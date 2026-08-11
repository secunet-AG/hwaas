// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::{
    ContextApiConfig,
    api::App,
    app_config::ContextMaxLifetimeSetting,
    tests::{
        network_api::{WS_GATEWAY_ADDR, WS_GATEWAY_URI},
        test_server,
    },
};

use std::time::Duration;
use std::{collections::HashSet, net::SocketAddr, ops::Range, sync::Arc};

use assert_fs::TempDir;

use context_data_structures::{aliases::ContextId, machine_properties::MachineProperties};

use db_interaction::{connection::DbFacade, test_utils::TestDb};
use futures::executor::block_on;
use machine_ops_lib::{initialization::InitializationOptions, machine_data::MachineData};
use net_ctrl_client_wrapper::NetCtrlClient;
use remote_client::RemoteClient;
use serde_json::{Value, json};
use tokio::{net::ToSocketAddrs, sync::oneshot::Sender, task::JoinHandle};
use tracing::trace;
use wiremock::{Mock, MockGuard, MockServer, ResponseTemplate, matchers::any};

fn default_properties() -> MachineProperties {
    MachineProperties {
        platform: String::from("SR630"),
    }
}

fn store_item_fixture(
    machine_id: usize,
    remote_mocks: &mut Vec<MockServer>,
    properties: MachineProperties,
) -> MachineData {
    let remote_mock = block_on(MockServer::start());
    let remote_address = remote_mock.uri().to_owned();
    remote_mocks.push(remote_mock);

    serde_json::from_value(json!(
      {
        "switch_connections": {
        "lan1": {
            "switch": "switch1",
            "port": TestServerSetup::switch_port(machine_id, 0),
        },
        "lan2": {
            "switch": "switche1",
            "port": TestServerSetup::switch_port(machine_id, 1),
        }

        },
        "platform": properties.platform,
        "state": "Free",
        "id": machine_id,
        "remote_auxiliary": format!("{remote_address}/auxiliaries"),
        "remote_serial": format!("{remote_address}/serial"),
        "remote_power": format!("{remote_address}/power"),
        "remote_usb": format!("{remote_address}/usb"),
    }
    ))
    .unwrap()
}

/// Builder to configure a test server running the context api with a
/// configurable number of machines, timeouts, lifetime and net
/// controller.
pub(super) struct TestServerSetup {
    available_network_ids: Range<u16>,
    timeout: u64,
    machines: Vec<MachineData>,
    net_ctrl_mock: MockServer,
    remote_mocks: Vec<MockServer>,
    default_remote_mock_response: bool,
    default_net_ctrl_mock_response: bool,
    context_management_timeout: u64,
    context_lifetime: u64,
    context_max_lifetime: u64,
}

impl Default for TestServerSetup {
    fn default() -> Self {
        let net_ctrl_mock = block_on(MockServer::start());
        let mut remote_mocks = Vec::new();
        let machines = vec![store_item_fixture(
            1,
            &mut remote_mocks,
            default_properties(),
        )];
        Self {
            // This means that network ids 2 and 3
            // may be used.
            available_network_ids: (2..4),
            timeout: 80,
            net_ctrl_mock,
            machines,
            remote_mocks,
            default_remote_mock_response: true,
            default_net_ctrl_mock_response: true,
            context_management_timeout: 10_000,
            context_lifetime: 15,
            context_max_lifetime: Duration::from(ContextMaxLifetimeSetting::default()).as_secs(),
        }
    }
}

impl TestServerSetup {
    /// Set the number of machines available to the context api.
    pub fn with_num_machines(num_machines: usize) -> Self {
        let mut remote_mocks = Vec::new();
        Self {
            machines: (1..=num_machines)
                .map(|machine_num| {
                    store_item_fixture(machine_num, &mut remote_mocks, default_properties())
                })
                .collect(),
            remote_mocks,
            ..Default::default()
        }
    }

    /// Create a [`TestServerSetup`] configured for a single machine with the given `properties`.
    ///
    /// More machines with user specified properties may be appended later by calling [`TestServerSetup::append_machine`].
    /// The machine will have `1` as its unique identifier.
    pub fn with_machine(properties: MachineProperties) -> Self {
        Self::with_machine_and_id(properties, 1)
    }

    /// Similar to [`Self::with_machine()`], but allows you to set its identifier.
    pub fn with_machine_and_id(properties: MachineProperties, machine_id: u16) -> Self {
        let mut remote_mocks = Vec::new();
        let machines = vec![store_item_fixture(
            machine_id as usize,
            &mut remote_mocks,
            properties,
        )];
        Self {
            machines,
            remote_mocks,
            ..Default::default()
        }
    }

    /// Append a machine with the given `properties` to the pool of machines available to the server.
    pub fn append_machine(mut self, properties: MachineProperties) -> Self {
        let machine_id = self
            .machines
            .iter()
            .map(|machine_data| machine_data.id as usize)
            .max()
            .unwrap_or(0);
        let store_item = store_item_fixture(machine_id + 1, &mut self.remote_mocks, properties);
        self.machines.push(store_item);
        self
    }

    /// Similar to [`Self::append_machine`], but lets you set the id of the machine.
    ///
    /// NOTE: care must be taken to ensure that the given id will be unique among
    /// all machines configured for the server.
    pub fn append_machine_with_id(
        mut self,
        properties: MachineProperties,
        machine_id: u16,
    ) -> Self {
        let store_item =
            store_item_fixture(machine_id as usize, &mut self.remote_mocks, properties);
        self.machines.push(store_item);
        self
    }

    /// By default all mock remote-hands services
    /// return a 200 to any request. This
    /// method disables that behavior.
    pub fn no_default_remote_mocks(mut self) -> Self {
        self.default_remote_mock_response = false;
        self
    }

    /// Starts the server and immediately reserves all machines for a single context.
    /// Each machine will be named "abmr<number>" where number starts at 1.
    ///
    /// Returns the address for the server together with the identifier
    /// of the single context.
    ///
    /// # Important
    ///
    /// The machine named "abmr<number>" does not necessarily correspond
    /// to the <number>'th entry in the server's machine store.
    pub async fn start_reserved(self) -> (TestServerOutputs, ContextId) {
        let rsd = {
            let machines: serde_json::Map<String, Value> = self
                .machines
                .iter()
                .enumerate()
                .map(|(idx, item)| {
                    let machine_name = format!("abmr{}", idx + 1);
                    let constraints = json!(
                        {
                            "platform": item.platform
                        }
                    );
                    (machine_name, constraints)
                })
                .collect();
            json!({
                "machines": machines
            })
        };

        let test_server_outputs = self.start().await;

        // When reserving a machine all switch ports are deactivated hence we need our net control mock to return an OK in this case.
        let _net_ctrl_mock_guard = Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .mount_as_scoped(&test_server_outputs.net_ctrl_mock)
            .await;
        // Also various remote hands services get called to turn off power and deconfigure various devices etc.
        let mut remote_mock_guards: Vec<MockGuard> = Vec::new();
        for mock_server in &test_server_outputs.remote_mocks {
            let guard = Mock::given(any())
                .respond_with(ResponseTemplate::new(200))
                .mount_as_scoped(mock_server)
                .await;
            remote_mock_guards.push(guard);
        }

        let context_id = super::context_management::reserve_context(test_server_outputs.addr, rsd)
            .await
            .unwrap();

        (test_server_outputs, context_id)
    }

    /// Set a timeout for the handlers in the context api.
    pub fn timeout(mut self, milliseconds: u64) -> Self {
        self.timeout = milliseconds;
        self
    }

    fn switch_port(machine_index: usize, port_index: usize) -> String {
        (2 * machine_index + port_index).to_string()
    }

    /// By default the mock net controller
    /// returns a 200 to any request. This
    /// method disables that behavior.
    pub fn no_default_net_ctrl_mock(mut self) -> Self {
        self.default_net_ctrl_mock_response = false;
        self
    }

    /// Set a timeout for the context management api.
    pub fn context_management_timeout(mut self, seconds: u64) -> Self {
        self.context_management_timeout = seconds;
        self
    }

    /// Set a timeout for the context lifetime
    pub fn context_lifetime(mut self, seconds: u64) -> Self {
        self.context_lifetime = seconds;
        self
    }

    /// Set the maximum lifetime for contexts
    pub fn context_max_lifetime(mut self, seconds: u64) -> Self {
        self.context_max_lifetime = seconds;
        self
    }

    async fn start_from_configuration(
        test_db: TestDb,
        image_api_dir: TempDir,
        config: ContextApiConfig,
        addr: impl ToSocketAddrs,
        net_ctrl_mock: MockServer,
        remote_mocks: Vec<MockServer>,
    ) -> TestServerOutputs {
        let app = App::prepare(config.clone())
            .await
            .expect("Should be possible to prepare app");
        let (server, addr, shutdown_signal) = test_server::serve_app_with_addr(app, addr).await;
        let server_task_handle = tokio::spawn(server);
        // Wait a bit for the server to start
        tokio::time::sleep(Duration::from_millis(10)).await;
        TestServerOutputs {
            test_db,
            addr,
            net_ctrl_mock,
            remote_mocks,
            image_dir: image_api_dir,
            shutdown_signal,
            server_task_handle,
            app_config: config,
        }
    }

    /// Apply all configurations.
    ///
    /// This is meant to be called prior to starting a new test server.
    async fn configure(&self) -> (TempDir, TestDb, ContextApiConfig) {
        let Self {
            available_network_ids,
            timeout,
            machines,
            net_ctrl_mock,
            remote_mocks,
            default_remote_mock_response,
            default_net_ctrl_mock_response,
            context_management_timeout,
            context_lifetime,
            context_max_lifetime,
        } = &self;

        let temp_dir = TempDir::new().unwrap();
        let image_path = temp_dir.path().to_path_buf();

        let _ = WS_GATEWAY_URI.set(format!("ws://{}", WS_GATEWAY_ADDR));
        let mut test_db = TestDb::spawn();

        // Upsert available network ids to the database
        machine_ops_lib::network_identifiers::upsert_network_ids(
            &available_network_ids
                .clone()
                .map(|id| id as i16)
                .collect::<Vec<_>>(),
            &mut test_db.conn,
        )
        .unwrap();
        // Make machines available. We Temporarily mock all services to succeed during this
        // phase
        {
            let mut mock_guards: Vec<_> = Vec::with_capacity(remote_mocks.len());
            for mock_server in remote_mocks {
                let guard = Mock::given(any())
                    .respond_with(ResponseTemplate::new(200))
                    .mount_as_scoped(mock_server)
                    .await;
                mock_guards.push(guard);
            }
            mock_guards.push(
                net_ctrl_mock
                    .register_as_scoped(Mock::given(any()).respond_with(ResponseTemplate::new(200)))
                    .await,
            );
            trace!(machines = ?machines, "Going to initialize machines");
            // Check that the machine ids are unique otherwise tests may
            // behave unexpectedly and debugging can be difficult.
            let mut machine_ids = HashSet::with_capacity(machines.len());
            for machine in machines {
                let machine_id = machine.id;
                if !machine_ids.insert(machine_id) {
                    dbg!(machine_id);
                    panic!(
                        "Attempt to declare multiple machines with the same id in test detected"
                    );
                }
            }
            let db_facade = DbFacade::new(test_db.file.to_str().unwrap(), 4)
                .await
                .unwrap();

            machine_ops_lib::initialization::initialize(
                machines.clone(),
                Arc::new(db_facade),
                NetCtrlClient::new(net_ctrl_mock.uri()),
                RemoteClient::default(),
                &InitializationOptions::default(),
            )
            .await
            .expect("Should be possible to initialize machine");
        }

        let config_json = json!(
          {
            "db_file_path": test_db.file.clone(),
            "net_ctrl_base_path": "http://localhost:8765",
            "image_api_settings": {
              "max_file_size": "128MiB",
              "store": image_path
            },
            "network_gateway": {
              "ws_gateway_url": WS_GATEWAY_URI.get().unwrap()
            },
            "request_timeouts": {
              "single_context_api": timeout,
              "context_management_api": context_management_timeout
            },
            "max_db_connections": 2,
            "context_lifetime": context_lifetime,
            "context_max_lifetime": context_max_lifetime
          }
        );

        let mut config: ContextApiConfig = serde_json::from_value(config_json).unwrap();
        config.net_ctrl_base_path = net_ctrl_mock.uri();

        // Assert that file size is loaded correctly from human readable format
        assert_eq!(
            config.image_api_settings.max_file_size.as_u64(),
            128 * 1024 * 1024
        );

        // Now set the default runtime mock behaviors if given.
        let default_response = |req: &wiremock::Request| -> ResponseTemplate {
            // If the auxiliaries endpoint is called with the GET method we return info for a single mock
            // auxiliary device.
            if req.method == wiremock::http::Method::GET
                && req.url.as_str().ends_with("/auxiliaries")
            {
                ResponseTemplate::new(200)
                    .set_body_json(json!([{"activation": true, "id": "mock-auxiliary-device"}]))
            } else {
                ResponseTemplate::new(200)
            }
        };
        if *default_remote_mock_response {
            for remote_mock in remote_mocks {
                remote_mock
                    .register(Mock::given(any()).respond_with(default_response))
                    .await;
            }
        }

        if *default_net_ctrl_mock_response {
            net_ctrl_mock
                .register(Mock::given(any()).respond_with(ResponseTemplate::new(200)))
                .await;
        }
        (temp_dir, test_db, config)
    }
    /// Spawn a new server for testing the ContextAPI.
    ///
    /// NOTE: This needs to be called from within a tokio runtime.
    pub async fn start(self) -> TestServerOutputs {
        let (image_api_dir, test_db, config) = self.configure().await;
        let Self {
            net_ctrl_mock,
            remote_mocks,
            ..
        } = self;

        Self::start_from_configuration(
            test_db,
            image_api_dir,
            config,
            "127.0.0.1:0",
            net_ctrl_mock,
            remote_mocks,
        )
        .await
    }
}

/// A handle to the task running the server
type ServerTaskHandle = JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>>;

/// Outputs produced by the test server
/// setup.
pub struct TestServerOutputs {
    /// The address of the test server.
    pub addr: SocketAddr,
    /// A mock net controller.
    pub net_ctrl_mock: MockServer,

    /// The mock remote-hands services,
    /// the i'th entry corresponds
    /// to the i'th entry in the
    /// machine store.
    pub remote_mocks: Vec<MockServer>,

    /// Unique database for the test. This database is deleted upon drop.
    pub test_db: TestDb,

    /// Directory for the image api. The directory is deleted upon drop.
    image_dir: TempDir,

    /// Configuration for the Context api application.
    app_config: ContextApiConfig,

    /// Signal to trigger a graceful shutdown of the server.
    shutdown_signal: Sender<()>,

    server_task_handle: ServerTaskHandle,
}

impl TestServerOutputs {
    /// Restarts the currently running server.
    ///
    /// Note: This needs to be called from within a tokio runtime.
    pub async fn restart_server(self) -> Self {
        let Self {
            addr,
            net_ctrl_mock,
            remote_mocks,
            test_db,
            image_dir,
            app_config,
            shutdown_signal,
            server_task_handle,
        } = self;
        // Shutdown the currently running server.
        shutdown_signal.send(()).unwrap();
        let _ = server_task_handle.await.unwrap();

        TestServerSetup::start_from_configuration(
            test_db,
            image_dir,
            app_config,
            addr,
            net_ctrl_mock,
            remote_mocks,
        )
        .await
    }
}
