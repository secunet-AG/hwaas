// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

use context_data_structures::network::TaggedMachineNetworkInterface;

use db_interaction::models::aliases::NetworkId;
use futures::{SinkExt, StreamExt};
use reqwest::Response;
use serde_json::{Value, json};
use test_log::test;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::{Error, Message};
use tracing_test::traced_test;
use wiremock::{Mock, ResponseTemplate, matchers::method};

use crate::{
    API_VERSION,
    tests::{TestServerSetup, test_server_setup::TestServerOutputs},
};
use axum::{
    Router,
    extract::{Path, WebSocketUpgrade},
    http::StatusCode,
};

// Address of dummy websocket gateway used in the test(s) that need it
pub(super) const WS_GATEWAY_ADDR: SocketAddr =
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8234);
// Depends on WS_GATEWAY_ADDR and gets populated by the TestSetup
pub(super) static WS_GATEWAY_URI: OnceLock<String> = OnceLock::new();

// Tests the following:
// - PUT followed by GET is the identity (assuming no errors)
// - DELETE followed by GET gives a 404 (NOT FOUND) status code
// - DELETE is idempotent
#[tokio::test(flavor = "multi_thread")]
#[traced_test]
async fn simple_network_lifecycle() {
    let (
        TestServerOutputs {
            test_db: _test_db,
            addr,
            net_ctrl_mock: _net_ctrl_mock,
            remote_mocks: _remote_mocks,
            ..
        },
        ctx_id,
        ..,
    ) = TestServerSetup::default().start_reserved().await;
    let network_setup: Value = json!(
      {
        "abmr1": {
          "lan1": {}
        }
      }
    );

    // Create a new network with a single interface
    let network_name = "foo";

    let resource_url =
        format!("http://{addr}/{API_VERSION}/contexts/{ctx_id}/networks/{network_name}");

    let client = reqwest::Client::new();
    let response = client
        .put(&resource_url)
        .json(&network_setup)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Check that GET returns the same network setup we uploaded

    let response = client.get(&resource_url).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let setup_from_get: Value = response.json().await.unwrap();
    assert_eq!(network_setup, setup_from_get);

    // Check that DELETE works
    let response = client.delete(&resource_url).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let response = client.get(&resource_url).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let response_json: Result<Value, _> = response.json().await;
    assert!(response_json.is_err());

    // Check that DELETE now returns NOT FOUND

    let response = client.delete(&resource_url).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[test(tokio::test(flavor = "multi_thread"))]
#[traced_test]
async fn interface_reuse_with_put() {
    let (
        TestServerOutputs {
            test_db: _test_db,
            addr,
            net_ctrl_mock: _net_ctrl_mock,
            remote_mocks: _remote_mocks,
            ..
        },
        ctx_id,
    ) = TestServerSetup::default().start_reserved().await;

    let setup: Value = json!(
      {
        "abmr1": {
          "lan1": {}
        }
      }
    );

    // Create this network setup for a network
    let network1 = "foo";
    let network1_url = format!("http://{addr}/{API_VERSION}/contexts/{ctx_id}/networks/{network1}");
    let client = reqwest::Client::new();

    assert!(
        client
            .put(&network1_url)
            .json(&setup)
            .send()
            .await
            .unwrap()
            .status()
            .is_success()
    );

    // Using the setup with another network should work.
    let network2 = "bar";
    let network2_url = format!("http://{addr}/{API_VERSION}/contexts/{ctx_id}/networks/{network2}");

    assert!(
        client
            .put(&network2_url)
            .json(&setup)
            .send()
            .await
            .unwrap()
            .status()
            .is_success()
    );

    // Check that `setup` is now associated with network2
    assert_eq!(
        client
            .get(&network2_url)
            .json(&setup)
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap(),
        setup
    );

    // Check that all interfaces described in the setup have now
    // been disconnected from network1.
    assert_eq!(
        client
            .get(&network1_url)
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap(),
        json!({})
    );
}

// Tests that one can update a network setup by submitting another PUT.
#[tokio::test(flavor = "multi_thread")]
async fn update_via_put() {
    let (
        TestServerOutputs {
            test_db: _test_db,
            addr,
            net_ctrl_mock: _net_ctrl_mock,
            remote_mocks: _remote_mocks,
            ..
        },
        ctx_id,
    ) = TestServerSetup::with_num_machines(2).start_reserved().await;

    let network_name = "foo";
    let network_setup: Value = json!(
      {
        "abmr1": {
          "lan1": {},
          "lan2": {}
        }
      }
    );

    let resource_url =
        format!("http://{addr}/{API_VERSION}/contexts/{ctx_id}/networks/{network_name}");

    let client = reqwest::Client::new();

    let response = client
        .put(&resource_url)
        .json(&network_setup)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // retrieve the setup
    let response = client.get(&resource_url).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(response.json::<Value>().await.unwrap(), network_setup);

    let new_setup: Value = json!
    (
     {
      "abmr1": {
        "lan2": {}
      },
      "abmr2": {
        "lan1": {}
      }
     }
    );
    let response = client
        .put(&resource_url)
        .json(&new_setup)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = client.get(&resource_url).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(new_setup, response.json::<Value>().await.unwrap());
}

// Send two different setups to the same Network. In the end the state should
// correspond to exactly one of them (the one that was handled last). Repeat the
// test a few times because randomness is involved.
#[test(tokio::test(flavor = "multi_thread"))]
async fn exclusive_put() {
    let (
        TestServerOutputs {
            test_db: _test_db,
            addr,
            net_ctrl_mock: _net_ctrl_mock,
            remote_mocks: _test_mocks,
            ..
        },
        ctx_id,
    ) = TestServerSetup::default().start_reserved().await;
    let client = reqwest::Client::new();

    for _ in 0..8 {
        let network_name = "foo";
        let setup_1: Value = json!(
          {
            "abmr1": {
              "lan1": {},
            }
          }
        );

        let setup_2: Value = json!(
          {
            "abmr1": {
              "lan2": {},
            }
          }
        );

        let resource_url =
            format!("http://{addr}/{API_VERSION}/contexts/{ctx_id}/networks/{network_name}");

        // Try to put both setups for the same network concurrently
        let mut fut_1 = std::pin::pin!(client.put(&resource_url).json(&setup_1).send());
        let mut fut_2 = std::pin::pin!(client.put(&resource_url).json(&setup_2).send());

        // We use select! rather than join! because it randomizes order
        let mut resp_1: Option<Response> = None;
        let mut resp_2: Option<Response> = None;
        while resp_1.is_none() || resp_2.is_none() {
            tokio::select! {
              r1 = &mut fut_1, if resp_1.is_none() => {resp_1 = Some(r1.unwrap());},
              r2 = &mut fut_2, if resp_2.is_none() => {resp_2 = Some(r2.unwrap());},
            }
        }

        // Assert that both requests returned OK responses.
        assert_eq!(resp_1.unwrap().status(), StatusCode::OK);
        assert_eq!(resp_2.unwrap().status(), StatusCode::OK);

        let response = client.get(&resource_url).send().await.unwrap();
        let retrieved_setup: Value = response.json().await.unwrap();
        assert!(retrieved_setup == setup_1 || retrieved_setup == setup_2);
        let delete_response = client.delete(&resource_url).send().await.unwrap();
        assert_eq!(delete_response.status(), StatusCode::OK);
    }
}

// This tests the behavior when put and delete error because of errors returned from net ctrl.
// A PUT followed by GET should return a network setup containing all the
// declared interfaces that the net ctrl could successfully connect as well as any previous
// interfaces the net ctrl failed to disconnect. Similarly a DELETE followed with a GET
// should return a network setup consisting of those interfaces that the net ctrl could not
// disconnect.
#[tokio::test(flavor = "multi_thread")]
async fn connection_error_handling() {
    let (
        TestServerOutputs {
            test_db: _test_db,
            addr,
            net_ctrl_mock,
            remote_mocks: _test_mocks,
            ..
        },
        ctx_id,
    ) = TestServerSetup::with_num_machines(3)
        .no_default_net_ctrl_mock()
        .start_reserved()
        .await;

    // Before we can declare proper mocks we need to figure out the ports of a couple
    // of machines. We will do this for "abmr1" and "abmr2".
    let port_extractor = |req: &wiremock::Request| -> usize {
        let path_parts = req.url.path().split('/').collect::<Vec<_>>();
        usize::from_str(path_parts[4]).unwrap()
    };

    let (abmr1_lan1_port, abmr2_lan1_port) = {
        // We send one request per machine in our context and use the mock net ctrl handler to
        // extract the port per machine for us.
        let observed_ports = Arc::new([AtomicUsize::new(usize::MAX), AtomicUsize::new(usize::MAX)]);
        let observed_ports_remote_clone = observed_ports.clone();
        let guard = net_ctrl_mock
            .register_as_scoped(Mock::given(method("PUT")).respond_with(
                move |req: &wiremock::Request| {
                    let port = port_extractor(req);
                    for maybe_set in observed_ports_remote_clone.iter() {
                        if maybe_set
                            .compare_exchange(usize::MAX, port, Ordering::AcqRel, Ordering::Relaxed)
                            .is_ok()
                        {
                            break;
                        }
                    }
                    ResponseTemplate::new(200)
                },
            ))
            .await;

        let guard2 = net_ctrl_mock
            .register_as_scoped(
                Mock::given(method("DELETE")).respond_with(ResponseTemplate::new(200)),
            )
            .await;

        let network = "tempNetwork";
        let url = format!("http://{addr}/{API_VERSION}/contexts/{ctx_id}/networks/{network}");

        let client = reqwest::Client::new();
        // Obtain the ports
        for i in [1, 2] {
            let setup = json!({
                format!("abmr{}",i): {
                    "lan1": {}
                }
            });
            assert!(
                client
                    .put(&url)
                    .json(&setup)
                    .send()
                    .await
                    .unwrap()
                    .status()
                    .is_success()
            );
        }
        // Delete the network as we no longer need it here
        assert!(
            client
                .delete(&url)
                .send()
                .await
                .unwrap()
                .status()
                .is_success()
        );
        drop(guard);
        drop(guard2);
        (
            observed_ports[0].load(Ordering::Acquire),
            observed_ports[1].load(Ordering::Acquire),
        )
    };

    // Now we declare some interfaces we want to use in our test and set up proper mocks
    // using the corresponding switch ports that we now know.
    let can_connect1: TaggedMachineNetworkInterface = TaggedMachineNetworkInterface {
        machine_name: "abmr1".to_owned(),
        interface: "lan1".to_owned(),
    };

    let cannot_connect: TaggedMachineNetworkInterface = TaggedMachineNetworkInterface {
        machine_name: "abmr2".to_owned(),
        interface: "lan1".to_owned(),
    };

    let can_connect2: TaggedMachineNetworkInterface = TaggedMachineNetworkInterface {
        machine_name: "abmr3".to_owned(),
        interface: "lan1".to_owned(),
    };

    let cannot_disconnect: TaggedMachineNetworkInterface = can_connect1.clone();

    let error_port_responder = |erroneous_port| {
        move |request: &wiremock::Request| {
            if port_extractor(request) == erroneous_port {
                ResponseTemplate::new(500)
            } else {
                ResponseTemplate::new(200)
            }
        }
    };

    // Error when removing abmr1/lan1
    Mock::given(method("DELETE"))
        .respond_with(error_port_responder(abmr1_lan1_port))
        .mount(&net_ctrl_mock)
        .await;

    // Error when adding abmr2/lan1
    Mock::given(method("PUT"))
        .respond_with(error_port_responder(abmr2_lan1_port))
        .mount(&net_ctrl_mock)
        .await;

    // Send the initial network setup to the server
    let network_name = "foo";

    let resource_url =
        format!("http://{addr}/{API_VERSION}/contexts/{ctx_id}/networks/{network_name}");

    // Note that the interface declared in this set can be connected, but cannot be disconnected
    let first_setup: Value = json!(
      {
        &can_connect1.machine_name: {
          &can_connect1.interface: {},
        }
      }
    );

    let client = reqwest::Client::new();

    assert_eq!(
        client
            .put(&resource_url)
            .json(&first_setup)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );

    // We will now update the setup via put. We attempt to
    // send a new setup with one interface that cannot be connected.
    let next_setup: Value = json!(
      {
        &can_connect2.machine_name: {
          &can_connect2.interface: {},
        },
        cannot_connect.machine_name: {
          cannot_connect.interface: {}
        }
      }
    );

    // The server should return an error code here
    assert!(
        client
            .put(&resource_url)
            .json(&next_setup)
            .send()
            .await
            .unwrap()
            .status()
            .is_server_error()
    );

    // Check that the current network setup now consists of can_connect1 and can_connect2.
    // The former is still part of the setup because it could not be disconnected.

    assert_eq!(
        client
            .get(&resource_url)
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap(),
        json!(
          {
            can_connect1.machine_name: {
              can_connect1.interface: {}
            },
            &can_connect2.machine_name: {
              &can_connect2.interface: {},
            },
          }
        )
    );

    // Check that when we delete the setup it does not succeed, and
    // the interface that cannot be disconnected remains
    assert!(
        client
            .delete(&resource_url)
            .send()
            .await
            .unwrap()
            .status()
            .is_server_error()
    );

    assert_eq!(
        client
            .get(&resource_url)
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap(),
        json!(
            {
            cannot_disconnect.machine_name: {
              cannot_disconnect.interface: {}
            }
          }
        )
    );
}

// This tests the behavior when PUT and DELETE time out due to net ctrl calls taking too long.
// When retrieving a network setup whose update timed out the state should contain
// all declared interfaces that already are connected, those that were connected before the timeout
// and those that that were previously connected, but did not get disconnected before the timeout.
#[test(tokio::test(flavor = "multi_thread"))]
async fn connection_timeouts() {
    // All handlers should timeout after 20 milliseconds
    let timeout = Duration::from_millis(70);
    let net_ctrl_handling_time_for_problematic_interfaces = timeout + Duration::from_millis(50);
    let can_connect = TaggedMachineNetworkInterface {
        machine_name: "abmr1".to_owned(),
        interface: "lan1".to_owned(),
    };
    let cannot_connect = TaggedMachineNetworkInterface {
        machine_name: "abmr3".to_owned(),
        interface: "lan1".to_owned(),
    };
    let cannot_disconnect: TaggedMachineNetworkInterface = can_connect.clone();

    let (
        TestServerOutputs {
            test_db: _test_db,
            addr,
            net_ctrl_mock,
            remote_mocks: _test_mocks,
            ..
        },
        ctx_id,
    ) = TestServerSetup::with_num_machines(3)
        .timeout(timeout.as_millis() as u64)
        .no_default_net_ctrl_mock()
        .start_reserved()
        .await;
    let put_mock = Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(200))
        .mount_as_scoped(&net_ctrl_mock)
        .await;

    // Send the initial network setup to the server
    let network_name = "foo";

    let resource_url =
        format!("http://{addr}/{API_VERSION}/contexts/{ctx_id}/networks/{network_name}");

    // Note that the interface declared in this set can be connected, but cannot be disconnected
    let first_setup: Value = json!(
      {
        can_connect.machine_name.clone(): {
          can_connect.interface.clone(): {},
        }
      }
    );

    let client = reqwest::Client::new();

    assert_eq!(
        client
            .put(&resource_url)
            .json(&first_setup)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );

    // We will now update the setup via put. We attempt to
    // send a new setup with one interface that cannot be connected.
    let next_setup: Value = json!(
      {
        cannot_connect.machine_name: {
          cannot_connect.interface: {}
        }
      }
    );

    // The server should return an error code here
    drop(put_mock);
    Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
        .mount(&net_ctrl_mock)
        .await;
    assert_eq!(
        client
            .put(&resource_url)
            .json(&next_setup)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::REQUEST_TIMEOUT
    );

    // After receiving a response for our request, the network setup should be done updating.
    // We verify this by waiting a bit before calling GET
    tokio::time::sleep(
        2 * net_ctrl_handling_time_for_problematic_interfaces + Duration::from_millis(50),
    )
    .await;

    // Check that the current network setup now consists of can_connect1. The former is
    // still part of the setup because it could not be disconnected.
    Mock::given(method("DELETE"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
        .mount(&net_ctrl_mock)
        .await;

    assert_eq!(
        client
            .get(&resource_url)
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap(),
        json!(
          {
            can_connect.machine_name: {
              can_connect.interface: {}
            }
          }
        )
    );

    // Try deleting the network and verify that it times out and our interface does not get removed.
    assert_eq!(
        client.delete(&resource_url).send().await.unwrap().status(),
        StatusCode::REQUEST_TIMEOUT
    );

    // Verify that no changes are being made to the network setup in the background after returning
    tokio::time::sleep(
        net_ctrl_handling_time_for_problematic_interfaces + Duration::from_millis(50),
    )
    .await;
    assert_eq!(
        client.delete(&resource_url).send().await.unwrap().status(),
        StatusCode::REQUEST_TIMEOUT
    );
    assert_eq!(
        client
            .get(&resource_url)
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap(),
        json!(
          {
            cannot_disconnect.machine_name: {
              cannot_disconnect.interface: {}
            }
          }
        )
    );
}

#[test(tokio::test(flavor = "multi_thread"))]
async fn invalid_network_setup_error_codes() {
    let (
        TestServerOutputs {
            test_db: _test_db,
            addr,
            net_ctrl_mock,
            remote_mocks: _remote_mocks,
            ..
        },
        ctx_id,
    ) = TestServerSetup::default()
        .no_default_net_ctrl_mock()
        .start_reserved()
        .await;
    Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&net_ctrl_mock)
        .await;

    let network_foo = "foo";

    let network_setup: Value = json!(
      {
        "unknownMachineName": {"lan1": {}}

      }
    );

    let network_foo_url =
        format!("http://{addr}/{API_VERSION}/contexts/{ctx_id}/networks/{network_foo}");

    let client = reqwest::Client::new();

    let put_response = client
        .put(&network_foo_url)
        .json(&network_setup)
        .send()
        .await
        .unwrap();
    assert_eq!(put_response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[test(tokio::test(flavor = "multi_thread"))]
async fn websocket_termination() {
    let (
        TestServerOutputs {
            test_db: _test_db,
            addr,
            net_ctrl_mock,
            remote_mocks: _remote_mocks,
            ..
        },
        ctx_id,
    ) = TestServerSetup::default()
        .no_default_net_ctrl_mock()
        .start_reserved()
        .await;
    Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&net_ctrl_mock)
        .await;

    let network = "foo";
    let resource_url = format!("http://{addr}/{API_VERSION}/contexts/{ctx_id}/networks/{network}");
    let client = reqwest::Client::new();

    // Create a network with no machines connected to it
    assert!(
        client
            .put(&resource_url)
            .json(&json!({}))
            .send()
            .await
            .unwrap()
            .status()
            .is_success()
    );

    // Check that GET returns an empty network setup
    assert_eq!(
        client
            .get(&resource_url)
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap(),
        json!({})
    );

    // Create a mock websocket gateway server
    // that echoes all messages.
    // NOTE: The code within these braces may be a bit fragile
    // with regards to refactoring as it relies on some internal
    // details of HWaaS context API network webcsocket handlers
    {
        async fn handle_connection(
            _: Path<NetworkId>,
            ws: WebSocketUpgrade,
        ) -> impl axum::response::IntoResponse {
            {
                ws.on_upgrade(move |mut socket| async move {
                    while let Some(Ok(msg)) = socket.recv().await {
                        let _ = socket.send(msg).await;
                    }
                })
            }
        }
        let ws_gateway_router: Router =
            Router::new().route("/ws/:net_id", axum::routing::get(handle_connection));
        let listener = TcpListener::bind(WS_GATEWAY_ADDR).await.unwrap();
        tokio::spawn(async move {
            axum::serve::serve(
                listener,
                ws_gateway_router.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
        });
        tokio::time::sleep(Duration::from_millis(5)).await;
    }

    // Check that we can connect to `network` via websockets
    let ws_url =
        format!("ws://{addr}/{API_VERSION}/contexts/{ctx_id}/networks/{network}/websocket");
    let ws_url_clone = ws_url.clone();
    let (ws, _) = tokio_tungstenite::connect_async(ws_url_clone)
        .await
        .unwrap();

    // Send a message through the websocket every 5 ms.
    let (mut sender, mut receiver) = ws.split();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(5)).await;
            if sender.send(Message::Ping(vec![1])).await.is_err() {
                break;
            }
        }
    });
    // After 50 ms we delete the network. Once we have
    // confirmation that the network was deleted we assert
    // that the websocket connection was terminated within
    // 20 ms after that.
    let called_delete = Arc::new(AtomicBool::new(false));
    let called_delete_clone = called_delete.clone();
    let resource_url_clone = resource_url.clone();
    let wait_delete_wait = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        called_delete_clone.store(true, Ordering::Release);
        assert!(
            client
                .delete(&resource_url_clone)
                .send()
                .await
                .unwrap()
                .status()
                .is_success()
        );

        tokio::time::sleep(Duration::from_millis(30)).await
    });

    let connection_closed_by_delete_call = tokio::select! {
        ret = async {
             while let Some(rec) = receiver.next().await {
              match rec {
                Ok(Message::Close(_)) => {break;},
                Err(e) => {
                  match e {
                    Error::ConnectionClosed | Error::AlreadyClosed => {
                      break;
                    },
                    _ => { dbg!(e); continue}
                  }
                },
                _ => continue
              }
          };
          // At this point we are no longer receiving messages from
          // the stream. We return true if we got here after the network delete
          // was sent.
          called_delete.load(Ordering::Acquire)
        } => ret,
      _ = wait_delete_wait => {
        false
      }
    };
    assert!(connection_closed_by_delete_call);

    // As a last check we check that GET now fails since the
    // network is deleted at this point.
    assert_eq!(
        reqwest::Client::new()
            .get(&resource_url)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
}

// Tests that we can retrieve all network names created for a given context.
#[test(tokio::test(flavor = "multi_thread"))]
async fn get_networks() {
    let (
        TestServerOutputs {
            test_db: _test_db,
            addr,
            net_ctrl_mock,
            remote_mocks: _remote_mocks,
            ..
        },
        ctx_id,
    ) = TestServerSetup::with_num_machines(2)
        .no_default_net_ctrl_mock()
        .start_reserved()
        .await;
    Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&net_ctrl_mock)
        .await;

    // expect an initially empty list of networks
    let resource_url = format!("http://{addr}/{API_VERSION}/contexts/{ctx_id}/networks");
    let client = reqwest::Client::new();
    let response = client.get(&resource_url).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let resp_json = response.json::<Value>().await.unwrap();
    assert_eq!(resp_json, json!([]));

    // Send the initial network setup to the server
    let network_name = "foo";
    let network_setup: Value = json!(
      {
        "abmr1": {
          "lan1": {},
          "lan2": {}
        }
      }
    );
    let resource_url =
        format!("http://{addr}/{API_VERSION}/contexts/{ctx_id}/networks/{network_name}");
    let client = reqwest::Client::new();
    assert_eq!(
        client
            .put(&resource_url)
            .json(&network_setup)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );

    // test if we obtain the created network name when querying all network names
    let resource_url = format!("http://{addr}/{API_VERSION}/contexts/{ctx_id}/networks");
    let response = client.get(&resource_url).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let resp_json = response.json::<Value>().await.unwrap();
    assert_eq!(resp_json, json!(["foo"]));
}

/// Routine: Create server with the `TEST_CONTEXT` and 4 available machines. Then create a network named `foo`
/// with `initial` as the corresponding network setup. Apply the given `patch` then retrieve the network
/// setup using GET and assert that it corresponds to `expected`.
///
/// If the const `EXPECT_SUCCESSFUL_PATCH_RESPONSE` we assert that a success response status code is received
/// from the call to PATCH, otherwise we assert that the status code indicates a client error.
async fn put_patch_get_assert<const EXPECT_SUCCESSFUL_PATCH_RESPONSE: bool>(
    initial_network_setup: Value,
    network_setup_patch: Value,
    expected_network_setup: Value,
) {
    let (
        TestServerOutputs {
            test_db: _test_db,
            addr,
            net_ctrl_mock: _net_ctrl_mock,
            remote_mocks: _remote_mocks,
            ..
        },
        ctx_id,
    ) = TestServerSetup::with_num_machines(4).start_reserved().await;
    // Send the initial network setup to the server
    let network_name = "foo";

    let resource_url =
        format!("http://{addr}/{API_VERSION}/contexts/{ctx_id}/networks/{network_name}");
    let client = reqwest::Client::new();
    assert!(
        client
            .put(&resource_url)
            .json(&initial_network_setup)
            .send()
            .await
            .unwrap()
            .status()
            .is_success()
    );

    let patch_response_status = client
        .patch(&resource_url)
        .json(&network_setup_patch)
        .send()
        .await
        .unwrap()
        .status();

    assert!(if EXPECT_SUCCESSFUL_PATCH_RESPONSE {
        patch_response_status.is_success()
    } else {
        patch_response_status.is_client_error()
    });

    let response = client.get(&resource_url).send().await.unwrap();
    assert!(response.status().is_success());

    assert_eq!(
        expected_network_setup,
        response.json::<Value>().await.unwrap()
    );
}

/// We expect a successful response when calling PATCH with an
/// applicable patch.
const APPLICABLE_PATCH_RESPONSE_SUCCESS_EXPECTATION: bool = true;

// Test that a network setup can have an interface added by using a patch
#[test(tokio::test(flavor = "multi_thread"))]
async fn add_interface_with_patch() {
    let initial_network_setup: Value = json!({});

    let patch: Value = json!(
        [
          {"op": "add", "path": "/abmr1", "value": {"lan1": {}}},
        ]
    );

    let expected_network_setup: Value = json!(
        {
        "abmr1": {"lan1": {}},
      }
    );

    put_patch_get_assert::<APPLICABLE_PATCH_RESPONSE_SUCCESS_EXPECTATION>(
        initial_network_setup,
        patch,
        expected_network_setup,
    )
    .await;
}

// Test that a network setup can have an interface removed by using a patch
#[test(tokio::test(flavor = "multi_thread"))]
async fn remove_interface_with_patch() {
    let initial_network_setup: Value = json!({
      "abmr1": {"lan1": {}}
    });

    let patch: Value = json!(
        [
          {"op": "remove", "path": "/abmr1/lan1"},
        ]
    );

    let expected_network_setup: Value = json!({});

    put_patch_get_assert::<APPLICABLE_PATCH_RESPONSE_SUCCESS_EXPECTATION>(
        initial_network_setup,
        patch,
        expected_network_setup,
    )
    .await;
}

// Test that a network setup can be updated using the patch method containing multiple operations
#[test(tokio::test(flavor = "multi_thread"))]
async fn multiple_operations_patch() {
    let initial_network_setup: Value = json!(
      {
        "abmr1": {
          "lan1": {},
          "lan2": {}
        },
        "abmr2": {
          "lan1": {},
          "lan2": {}
        }
      }
    );

    let patch: Value = json!(
        [
          {"op": "add", "path": "/abmr3", "value": {"lan1": {}}},
          {"op": "remove", "path": "/abmr1"},
          {"op": "remove", "path": "/abmr2/lan1"},
        ]
    );

    // Check that the patch applied
    let expected_network_setup: Value = json!(
        {
        "abmr2": {"lan2": {}},
        "abmr3": {"lan1": {}},
      }
    );

    put_patch_get_assert::<APPLICABLE_PATCH_RESPONSE_SUCCESS_EXPECTATION>(
        initial_network_setup,
        patch,
        expected_network_setup,
    )
    .await;
}

// Test that the operations of a network setup patch are applied in order
// when it matters.
#[test(tokio::test(flavor = "multi_thread"))]
async fn patch_order_is_respected() {
    let initial_network_setup: Value = json!({});

    let patch: Value = json!(
        [
          {"op": "add", "path": "/abmr1", "value": {"lan1": {}}},
          {"op": "remove", "path": "/abmr1/lan1"}
        ]
    );

    let expected_network_setup: Value = initial_network_setup.clone();

    put_patch_get_assert::<APPLICABLE_PATCH_RESPONSE_SUCCESS_EXPECTATION>(
        initial_network_setup,
        patch,
        expected_network_setup,
    )
    .await;
}

// Test that an empty patch has no effect on a network setup
#[test(tokio::test(flavor = "multi_thread"))]
async fn empty_patch_has_no_effect() {
    let initial_network_setup: Value = json!(
      {
        "abmr1": {
          "lan1": {},
          "lan2": {}
        },
        "abmr2": {
          "lan1": {},
          "lan2": {}
        }
      }
    );

    let patch: Value = json!([]);

    let expected_network_setup: Value = initial_network_setup.clone();

    put_patch_get_assert::<APPLICABLE_PATCH_RESPONSE_SUCCESS_EXPECTATION>(
        initial_network_setup,
        patch,
        expected_network_setup,
    )
    .await;
}

/// We do not expect a successful response when calling PATCH with an
/// unapplicable patch.
const UNAPPLICABLE_PATCH_RESPONSE_SUCCESS_EXPECTATION: bool = false;

#[test(tokio::test(flavor = "multi_thread"))]
async fn invalid_remove_non_existing_machine() {
    let initial_network_setup: Value = json!({});
    let patch: Value = json!([{"op": "remove", "path": "/abmr1"}]);
    put_patch_get_assert::<UNAPPLICABLE_PATCH_RESPONSE_SUCCESS_EXPECTATION>(
        initial_network_setup.clone(),
        patch,
        initial_network_setup,
    )
    .await;
}

#[test(tokio::test(flavor = "multi_thread"))]
async fn invalid_add_interface_json_ptr_with_parent_ptr_not_in_setup() {
    let initial_network_setup: Value = json!({});
    let patch: Value = json!([{"op": "add", "path": "/abmr1/lan1", "value": {}}]);
    // NOTE: patch = {op: add, path: /abmr1, value: {lan1: {}}} would work
    put_patch_get_assert::<UNAPPLICABLE_PATCH_RESPONSE_SUCCESS_EXPECTATION>(
        initial_network_setup.clone(),
        patch,
        initial_network_setup,
    )
    .await;
}

#[test(tokio::test(flavor = "multi_thread"))]
async fn invalid_add_machine_not_in_setup() {
    let initial_network_setup: Value = json!({});
    let patch: Value = json!([{"op": "add", "path": "/abmr1000", "value": {"lan1": {}}}]);
    // NOTE: Does not work because we know that machine "abmr1000" is not part of the context
    put_patch_get_assert::<UNAPPLICABLE_PATCH_RESPONSE_SUCCESS_EXPECTATION>(
        initial_network_setup.clone(),
        patch,
        initial_network_setup,
    )
    .await;
}

#[test(tokio::test(flavor = "multi_thread"))]
async fn interface_reuse_in_patch() {
    let (
        TestServerOutputs {
            test_db: _test_db,
            addr,
            net_ctrl_mock: _net_ctrl_mock,
            remote_mocks: _remote_mocks,
            ..
        },
        ctx_id,
    ) = TestServerSetup::with_num_machines(2).start_reserved().await;

    let network1 = "foo";
    let network1_url = format!("http://{addr}/{API_VERSION}/contexts/{ctx_id}/networks/{network1}");

    // push a setup consisting of a single interface to network1
    let machine = "abmr1";
    let interface = "lan1";

    let client = reqwest::Client::new();

    let setup: Value = json!({
      machine: {interface: {}}
    });

    assert!(
        client
            .put(&network1_url)
            .json(&setup)
            .send()
            .await
            .unwrap()
            .status()
            .is_success()
    );

    // Create a new network with an empty network setup
    let network2 = "bar";
    let network2_url = format!("http://{addr}/{API_VERSION}/contexts/{ctx_id}/networks/{network2}");

    let _ = client.put(&network2_url).json(&json!({})).send().await;

    // Now apply a patch adding the interface described above to network2
    let patch: Value = json!([{
      "op": "add", "path": format!("/{machine}"), "value": {interface: {}}
    }]);

    assert!(
        client
            .patch(&network2_url)
            .json(&patch)
            .send()
            .await
            .unwrap()
            .status()
            .is_success()
    );

    // Now `setup` should correspond to the state of network2 and network1 should now be empty
    assert_eq!(
        client
            .get(&network2_url)
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap(),
        setup
    );

    assert_eq!(
        client
            .get(&network1_url)
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap(),
        json!({})
    );
}
