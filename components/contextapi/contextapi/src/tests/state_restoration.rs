// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::HashSet,
    time::{Duration, Instant},
};

use context_data_structures::{aliases::ContextId, machine_properties::MachineProperties};
use reqwest::StatusCode;
use serde_json::{json, Value};

use super::test_server_setup::{TestServerOutputs, TestServerSetup};

const PLATFORM1: &str = "somePlatform";
const PLATFORM2: &str = "otherPlatform";

const MACHINE_NAME1: &str = "server";
const MACHINE_NAME2: &str = "client";

const API_VERSION: &str = crate::API_VERSION;
const NETWORK_NAME: &str = "foo";

#[tokio::test(flavor = "multi_thread")]
async fn context_is_available_after_restart() {
    let server_outputs = common_test_server_setup().start().await;
    // Reserve a context and setup a network between its reserved machines.
    let client = reqwest::Client::new();
    let context_id = server_outputs
        .put_test_configuration(&client)
        .await
        .unwrap();
    // Restart the test server.
    let server_outputs = server_outputs.restart_server().await;
    // Check that the context state is unaltered
    server_outputs
        .check_test_configuration(context_id, &client)
        .await;
    let addr = server_outputs.addr;
    // As a last check we ensure that we can delete the existing network
    let network_url =
        format!("http://{addr}/{API_VERSION}/contexts/{context_id}/networks/{NETWORK_NAME}");
    assert!(reqwest::Client::new()
        .delete(&network_url)
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
}

// Check that restarting the server does not make
// context timeouts disappear.
#[tokio::test(flavor = "multi_thread")]
async fn context_timeout_spawned_at_startup() {
    const CONTEXT_TIMEOUT: u64 = 3;
    let server_outputs = common_test_server_setup()
        .context_lifetime(CONTEXT_TIMEOUT)
        .start()
        .await;
    let client = reqwest::Client::new();
    // Reserve a context and setup a network between its reserved machines.
    let instant_before_ctx_reservation = Instant::now();
    let context_id = server_outputs
        .put_test_configuration(&client)
        .await
        .unwrap();
    // Restart the test server.
    let server_outputs = server_outputs.restart_server().await;
    let duration_since_restart = Instant::now().duration_since(instant_before_ctx_reservation);
    // We are assuming this should complete before the context timeout duration has passed. Adjust
    // the time if the test becomes flaky.
    assert!(duration_since_restart.as_secs() < CONTEXT_TIMEOUT);
    // Check that the context still exists.
    let addr = server_outputs.addr;
    let context_url = format!("http://{addr}/{API_VERSION}/contexts/{context_id}");
    let remaining_context_lifetime = client
        .get(&context_url)
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap()
        .get("lifetime")
        .unwrap()
        .as_u64()
        .unwrap();
    // We are assuming the reset can be completed within 2 seconds. Adjust the time if this becomes
    // flaky.
    tokio::time::sleep(Duration::from_secs(remaining_context_lifetime + 2)).await;
    // The context should now no longer be available.
    assert_eq!(
        StatusCode::NOT_FOUND,
        client.get(&context_url).send().await.unwrap().status(),
    );
}

/// RSD used in the tests of this module.
fn rsd() -> Value {
    json!(
        {
            "machines": {
                MACHINE_NAME1: {
                    "platform": PLATFORM1
                },
                MACHINE_NAME2: {
                    "platform": PLATFORM2
                }
            }
        }
    )
}

/// Network setup used in the tests of this module.
fn network_setup() -> Value {
    json!(
        {
            MACHINE_NAME1:{
                "lan1": {}
            },
            MACHINE_NAME2: {
                "lan2": {

                }
            }
        }
    )
}

/// Returns a TestServerSetup that has been configured to
/// have two machines of [`PLATFORM1`] and [`PLATFORM2`]
/// respectively.
fn common_test_server_setup() -> TestServerSetup {
    TestServerSetup::with_machine(MachineProperties {
        platform: PLATFORM1.to_string(),
    })
    .append_machine(MachineProperties {
        platform: PLATFORM2.to_string(),
    })
}

impl TestServerOutputs {
    /// Attempts to reserve a context according to `rsd()` and setup a network configured by `network_setup()`.
    async fn put_test_configuration(
        &self,
        client: &reqwest::Client,
    ) -> Result<ContextId, StatusCode> {
        let addr = self.addr;
        let rsd = rsd();
        let ctx_id =
            super::context_management::reserve_context_with_client(addr, rsd, client).await?;

        // Create a new network

        let network_url =
            format!("http://{addr}/{API_VERSION}/contexts/{ctx_id}/networks/{NETWORK_NAME}");

        let response = client
            .put(&network_url)
            .json(&network_setup())
            .send()
            .await
            .expect("Should not get client error");
        let status = response.status();
        status.is_success().then_some(ctx_id).ok_or(status)
    }

    /// Retrieves machine and network data concerning the context from the server and
    /// asserts that it agrees with what was put with [`Self::put_test_configuration`].
    async fn check_test_configuration(&self, context_id: ContextId, client: &reqwest::Client) {
        let addr = self.addr;
        let machines_url = format!("http://{addr}/{API_VERSION}/contexts/{context_id}/machines");
        // Check that the context contains exactly two machines named MACHINE1 and MACHINE2
        let machines = client
            .get(&machines_url)
            .send()
            .await
            .unwrap()
            .json::<Vec<String>>()
            .await
            .unwrap();
        assert_eq!(
            machines.into_iter().collect::<HashSet<String>>(),
            [MACHINE_NAME1, MACHINE_NAME2]
                .into_iter()
                .map(&str::to_string)
                .collect::<HashSet<String>>()
        );

        // Now check that there is exactly one network for the context.
        let networks_url = format!("http://{addr}/{API_VERSION}/contexts/{context_id}/networks");
        let networks = client
            .get(&networks_url)
            .send()
            .await
            .unwrap()
            .json::<Vec<String>>()
            .await
            .unwrap();
        assert_eq!(&networks, &[NETWORK_NAME]);

        // Check that the network setup corresponds to `network_setup()`.
        let network_url = format!("{networks_url}/{NETWORK_NAME}");
        let network = client
            .get(&network_url)
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap();
        assert_eq!(network, network_setup());
    }
}
