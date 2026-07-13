// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::tests::test_server_setup::{TestServerOutputs, TestServerSetup};
use crate::API_VERSION;
use assert_json_diff::assert_json_eq;
use assert_json_diff::assert_json_include;
use context_data_structures::machine_properties::MachineProperties;
use serde_json::{json, Value};
use std::time::Duration;
use tokio::time::sleep;
use tracing::debug;
use tracing_test::traced_test;

static PLATFORM_A: &str = "platform-a";
static PLATFORM_B: &str = "platform-b";
const ESTIMATED_RESERVATION_DURATION_KEY: &str = "estimated_reservation_duration";
const MACHINE_ID_ONE: u16 = 1;
const MACHINE_ID_TWO: u16 = 2;

fn test_server_with_machines() -> TestServerSetup {
    TestServerSetup::with_machine_and_id(
        MachineProperties {
            platform: PLATFORM_A.to_string(),
        },
        MACHINE_ID_ONE,
    )
    .append_machine_with_id(
        MachineProperties {
            platform: PLATFORM_B.to_string(),
        },
        MACHINE_ID_TWO,
    )
}

fn expected_inventory_response_approximation(all_free: bool) -> Value {
    let state = if all_free { "free" } else { "reserved" };
    json!([
        {
            "properties": { "platform": PLATFORM_A },
            "state": state,
            "machine_id": MACHINE_ID_ONE
        },
        {
            "properties": { "platform": PLATFORM_B },
            "state": state,
            "machine_id": MACHINE_ID_TWO
        }
    ])
}

#[tokio::test(flavor = "multi_thread")]
#[traced_test]
async fn get_machines_all_free() {
    let TestServerOutputs { addr, .. } = test_server_with_machines().start().await;

    // make the inventory request
    let resource_url = format!("http://{addr}/{API_VERSION}/inventory");
    debug!("FOO");
    let client = reqwest::ClientBuilder::new()
        .build()
        .inspect_err(|e| panic!("failed to build client: {e}"))
        .unwrap();
    let response = client.get(resource_url).send().await.unwrap();

    let expected_response = expected_inventory_response_approximation(true);

    assert!(response.status().is_success());
    assert_json_eq!(expected_response, response.json::<Value>().await.unwrap());
}

#[tokio::test(flavor = "multi_thread")]
#[traced_test]
async fn get_machines_all_reserved_and_free_again() {
    let (
        TestServerOutputs {
            addr,
            remote_mocks: _remote_mocks,
            net_ctrl_mock: _net_ctrl_mock,
            ..
        },
        ctx_id,
    ) = test_server_with_machines().start_reserved().await;

    // make the inventory request
    let resource_url = format!("http://{addr}/{API_VERSION}/inventory");
    let client = reqwest::Client::new();
    let response = client.get(resource_url.clone()).send().await.unwrap();

    // all machines should be reserved
    let expected_response = expected_inventory_response_approximation(false);
    assert!(response.status().is_success());
    assert_json_include!(expected: expected_response, actual: response.json::<Value>().await.unwrap());

    // delete the context in order to free all reserved ones
    let url = format!("http://{addr}/{API_VERSION}/contexts/{ctx_id}");
    assert!(client
        .delete(&url)
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    sleep(Duration::from_secs(5)).await;

    // test if all machines are displayed as free now
    let expected_response = expected_inventory_response_approximation(true);
    let response = client.get(resource_url).send().await.unwrap();
    assert!(response.status().is_success());
    assert_json_eq!(expected_response, response.json::<Value>().await.unwrap());
}

// Test that machine reservation estimates displayed in the inventory work as expected
#[tokio::test(flavor = "multi_thread")]
async fn get_machines_all_reserved_reservation_estimates() {
    let (TestServerOutputs { addr, .. }, ctx_id) =
        test_server_with_machines().start_reserved().await;

    // make the inventory request
    let resource_url = format!("http://{addr}/{API_VERSION}/inventory");
    let client = reqwest::Client::new();
    let response = client.get(resource_url.clone()).send().await.unwrap();

    // all machines should be reserved
    let expected_response_approximation = expected_inventory_response_approximation(false);
    assert!(response.status().is_success());
    let actual_response_json = response.json::<Value>().await.unwrap();
    assert_json_include!(expected: &expected_response_approximation, actual: &actual_response_json);
    assert!(actual_response_json
        .as_array()
        .unwrap()
        .iter()
        .all(|value| value.get(ESTIMATED_RESERVATION_DURATION_KEY).is_some()));

    // Patch the context to set a timeout 100 seconds from now
    let timeout_seconds = 100;

    let context_resource_url = format!("http://{addr}/{API_VERSION}/contexts/{ctx_id}");
    let context_patch = json!({
    "lifetime": timeout_seconds,
    });

    assert!(reqwest::Client::new()
        .patch(context_resource_url)
        .json(&context_patch)
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    // Sleep a bit in order to give the server some time to update the context timeout.
    sleep(Duration::from_secs(3)).await;

    // Check that the machines have an estimated reservation duration close to 100 seconds
    let response = client.get(resource_url).send().await.unwrap();
    assert!(response.status().is_success());
    let actual_response_json = response.json::<Value>().await.unwrap();
    // This is just a small reality check that the machine's have not changed and that they are still reserved.
    let expected_response_approximation = expected_inventory_response_approximation(false);
    assert_json_include!(
        expected: &expected_response_approximation,
        actual: &actual_response_json
    );
    // We check that the estimated reservation duration is at most 10 seconds away to the 100 seconds
    // we asked for when patching our context.
    assert!(actual_response_json
        .as_array()
        .unwrap()
        .iter()
        .all(|machine_data| {
            (machine_data
                .get(ESTIMATED_RESERVATION_DURATION_KEY)
                .unwrap()
                .as_i64()
                .unwrap()
                - 100)
                .unsigned_abs()
                <= 10
        }));
}
