// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use axum::http::HeaderMap;
use reqwest::header::CONTENT_TYPE;
use serde_json::{json, Value};
use tracing_test::traced_test;
use wiremock::matchers::{any, method};
use wiremock::{Mock, Request, ResponseTemplate};

use crate::tests::test_server_setup::TestServerSetup;
use crate::API_VERSION;
use reqwest::StatusCode;
use test_log::test;
use tracing::log::error;

// Test the GET method for the Machines API
#[tokio::test(flavor = "multi_thread")]
async fn get_machines() {
    let (test_outputs, ctx_id) = TestServerSetup::default().start_reserved().await;
    let addr = test_outputs.addr;
    let resource_url = format!("http://{addr}/{API_VERSION}/contexts/{ctx_id}/machines");
    let client = reqwest::Client::new();
    let response = client.get(resource_url).send().await.unwrap();
    assert!(response.status().is_success());
    // The test server is configured with a single machine "abmr1"
    assert_eq!(json!(["abmr1"]), response.json::<Value>().await.unwrap());
}

// Test that one can get a list of the network interfaces
// of a given machine using GET
#[tokio::test(flavor = "multi_thread")]
#[traced_test]
async fn get_network_interfaces_of_machine() {
    let (test_setup_outputs, ctx_id) = TestServerSetup::default().start_reserved().await;
    let addr = test_setup_outputs.addr;
    let resource_url =
        format!("http://{addr}/{API_VERSION}/contexts/{ctx_id}/machines/abmr1/network-interfaces");
    let client = reqwest::Client::new();
    let response = client.get(resource_url).send().await.unwrap();
    assert_eq!(StatusCode::OK, response.status());
    // The test server is configured with a single machine "abmr1" with two interfaces "lan1" and "lan2"
    let expected_interfaces: HashSet<String> = ["lan1", "lan2"]
        .into_iter()
        .map(ToOwned::to_owned)
        .collect();
    assert_eq!(
        expected_interfaces,
        response.json::<HashSet<String>>().await.unwrap()
    );
}

/// The machine API forwards some requests directly to the machine's
/// remote-hands. We check that given an endpoint that should be unknown
/// to the remote-hands we get a NOT FOUND status. As our unit tests cannot
/// rely on available remote-hands we mock them here.
#[tokio::test]
async fn remote_user_errors() {
    let (test_setup_outputs, ctx_id) = TestServerSetup::default()
        .no_default_remote_mocks()
        .start_reserved()
        .await;
    let addr = test_setup_outputs.addr;

    const LOCATION_UNKNOWN_TO_TS: &str = "normal";
    let remote_mock_handler = Mock::given(method("GET")).respond_with(|req: &Request| {
        if req.url.as_str().ends_with(LOCATION_UNKNOWN_TO_TS) {
            ResponseTemplate::new(404)
        } else {
            ResponseTemplate::new(200)
        }
    });

    test_setup_outputs
        .remote_mocks
        .first()
        .unwrap()
        .register(remote_mock_handler)
        .await;

    // Test if internal error are kept internal
    let url = format!(
        "http://{addr}/{API_VERSION}/contexts/{ctx_id}/machines/abmr1/{LOCATION_UNKNOWN_TO_TS}"
    );

    let res = reqwest::Client::new().get(url).send().await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

/// The machine API forwards some requests directly to the machine's
/// remote-hands. In this test we assert that errors returned from the
/// remote-hands that should be kept internal are not directly
/// forwarded to the user.
#[test(tokio::test)]
async fn map_remote_errors_to_internal_errors() {
    let (test_setup, ctx_id) = TestServerSetup::default()
        .no_default_remote_mocks()
        .start_reserved()
        .await;
    let addr = test_setup.addr;
    // Pick a route that forwards a partial path to a remote-hands peripheral
    const INTERNAL_ERROR_SEGMENT: &str = "serial/inexistant";
    let remote_mock_hander = Mock::given(method("GET")).respond_with(|req: &Request| {
        if req.url.as_str().ends_with(INTERNAL_ERROR_SEGMENT) {
            ResponseTemplate::new(StatusCode::SERVICE_UNAVAILABLE)
        } else {
            ResponseTemplate::new(200)
        }
    });

    test_setup
        .remote_mocks
        .first()
        .unwrap()
        .register(remote_mock_hander)
        .await;

    // Test if internal error are kept internal
    let url = format!(
        "http://{addr}/{API_VERSION}/contexts/{ctx_id}/machines/abmr1/{INTERNAL_ERROR_SEGMENT}"
    );

    let res = reqwest::Client::new().get(url).send().await.unwrap();
    assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

/// Test sending an HTTP request to the auxiliary endpoint.
#[tokio::test]
#[traced_test]
async fn pass_full_aux_response() {
    let (test_setup_output, ctx_id) = TestServerSetup::default()
        .no_default_remote_mocks()
        .start_reserved()
        .await;
    let addr = test_setup_output.addr;

    // We will have one auxiliary device named "image"
    let configured_auxiliary_devices = json!([{
        "id": "image",
        "activation": true
    }]);

    let configured_auxiliary_devices_clone = configured_auxiliary_devices.clone();

    // Setup a mock handler that returns our configured list on GET /auxiliaries
    // and gives an insufficient storage status code when attempting a PUT to
    // auxiliaries/image/api which is the url for the image auxiliary device's API.
    // Attempts to reset the device should give 200 OK.
    let mock_handler = Mock::given(any()).respond_with(move |req: &Request| {
        if (req.method == wiremock::http::Method::GET) && req.url.path() == "/auxiliaries" {
            ResponseTemplate::new(200).set_body_json(configured_auxiliary_devices_clone.clone())
        } else if (req.method == wiremock::http::Method::PUT)
            && req.url.path() == "/auxiliaries/image/api"
        {
            ResponseTemplate::new(StatusCode::INSUFFICIENT_STORAGE)
                .insert_header("content-type", "image/png")
        } else if (req.method == wiremock::http::Method::POST)
            && req.url.path() == "/auxiliaries/reset"
        {
            ResponseTemplate::new(200)
        } else {
            ResponseTemplate::new(404)
        }
    });

    let mock_server = test_setup_output.remote_mocks.first().unwrap();
    mock_server.register(mock_handler).await;

    let aux_device_list_url =
        format!("http://{addr}/{API_VERSION}/contexts/{ctx_id}/machines/abmr1/auxiliaries");

    let client = reqwest::Client::new();
    let devices = client
        .get(aux_device_list_url)
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await;
    dbg!(&devices);

    assert_eq!(configured_auxiliary_devices, devices.unwrap());

    let aux_device_url = format!(
        "http://{addr}/{API_VERSION}/contexts/{ctx_id}/machines/abmr1/auxiliaries/image/api"
    );

    // Test if any aux response pass
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "image/png".parse().unwrap());

    let res = client
        .put(aux_device_url)
        .headers(headers.clone())
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::INSUFFICIENT_STORAGE);
    assert_eq!(res.headers().get(CONTENT_TYPE), headers.get(CONTENT_TYPE));

    // Finally check that we can access the reset endpoint for the machine's auxiliary devices
    let reset_url =
        format!("http://{addr}/{API_VERSION}/contexts/{ctx_id}/machines/abmr1/auxiliaries/reset");
    assert!(client
        .post(reset_url)
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
}

/// Test sending an HTTP GET request to query the remote-usb
/// configuration.
#[test(tokio::test(flavor = "multi_thread"))]
async fn get_usb() {
    let (test_setup_output, ctx_id) = TestServerSetup::default()
        .no_default_remote_mocks()
        .start_reserved()
        .await;
    let addr = test_setup_output.addr;
    let remote_mock_handler = Mock::given(method("GET")).respond_with(|req: &Request| {
        if req.url.as_str().ends_with("usb") {
            ResponseTemplate::new(StatusCode::OK).set_body_json(json!([{
                "type": "serial",
            }]))
        } else {
            ResponseTemplate::new(404)
        }
    });

    test_setup_output
        .remote_mocks
        .first()
        .unwrap()
        .register(remote_mock_handler)
        .await;

    let url = format!("http://{addr}/{API_VERSION}/contexts/{ctx_id}/machines/abmr1/usb");

    // Test if any usb response pass
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());

    let res = reqwest::Client::new().get(url).send().await.unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}

/// Test sending an HTTP PUT request to configure the remote-usb
/// endpoint.
#[tracing_test::traced_test]
#[test(tokio::test(flavor = "multi_thread"))]
async fn put_usb() {
    let (test_setup_output, ctx_id) = TestServerSetup::default()
        .no_default_remote_mocks()
        .start_reserved()
        .await;
    let addr = test_setup_output.addr;
    let remote_mock_handler = Mock::given(method("PUT")).respond_with(|req: &Request| {
        if req.url.as_str().ends_with("usb") {
            ResponseTemplate::new(StatusCode::OK).set_body_json(json!([{
                "type": "serial",
            }]))
        } else {
            ResponseTemplate::new(404)
        }
    });

    test_setup_output
        .remote_mocks
        .first()
        .unwrap()
        .register(remote_mock_handler)
        .await;

    let url = format!("http://{addr}/{API_VERSION}/contexts/{ctx_id}/machines/abmr1/usb");

    // Test if any aux response pass
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());

    let res = reqwest::Client::new()
        .put(url)
        .json(&json!([{
            "type": "serial",
        }]))
        .send()
        .await
        .unwrap();

    assert_eq!(res.headers().get(CONTENT_TYPE), headers.get(CONTENT_TYPE));
    let status = res.status();

    if status != StatusCode::OK {
        error!("### Resp: {}", res.text().await.unwrap());
    }

    assert_eq!(status, StatusCode::OK);
}

#[tracing_test::traced_test]
#[tokio::test(flavor = "multi_thread")]
async fn put_power_interface() {
    let (test_setup_output, ctx_id) = TestServerSetup::default().start_reserved().await;
    let device_id = String::from("power1");
    let addr = test_setup_output.addr;
    let url =
        format!("http://{addr}/{API_VERSION}/contexts/{ctx_id}/machines/abmr1/power/{device_id}");

    assert!(reqwest::Client::new()
        .put(url)
        .send()
        .await
        .unwrap()
        .status()
        .is_success());

    // Check mock for expected request
    let req = test_setup_output
        .remote_mocks
        .first()
        .unwrap()
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .find(|req| {
            dbg!(req);
            req.url
                .to_string()
                .ends_with(format!("power/{device_id}").as_str())
        })
        .unwrap();
    assert_eq!(req.method.to_string().to_lowercase(), "put");
}

// Test the GET method for the Machine Info API
#[tokio::test(flavor = "multi_thread")]
async fn get_machine_info() {
    let (test_outputs, ctx_id) = TestServerSetup::default().start_reserved().await;
    let addr = test_outputs.addr;
    let resource_url = format!("http://{addr}/{API_VERSION}/contexts/{ctx_id}/machines/abmr1");
    let client = reqwest::Client::new();
    let response = client.get(resource_url).send().await.unwrap();
    // Endpoint is reachable
    assert!(response.status().is_success());
    // Test if there is an id and platform field in the json part of the response
    let json_data: Value = response.json().await.unwrap();
    assert!(json_data.get("id").is_some());
    assert!(json_data.get("platform").is_some())
}
