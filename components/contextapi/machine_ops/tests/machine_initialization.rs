// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use assert_cmd::prelude::*;
use assert_fs::NamedTempFile;
use db_interaction::test_utils::TestDb;
use serde_json::{json, Value};
use std::process::Command;
use wiremock::{matchers::any, Mock, MockServer, ResponseTemplate};

const AUXILIARY_DEVICE_NAME: &str = "aux-1";

/// Returns an array of machine data consisting of a single entry
fn machines_json(
    remote_usb_address: String,
    remote_power_address: String,
    remote_auxiliary_address: String,
    remote_serial_address: String,
) -> Value {
    json!(
            [
                {
                    "id": 1,
                    "platform": "SR630",
                    "state": "Deactivated",
                    "remote_usb": remote_usb_address,
                    "remote_power": remote_power_address,
                    "remote_auxiliaries": {
                        AUXILIARY_DEVICE_NAME: remote_auxiliary_address
                    },
                    "remote_serial": remote_serial_address,
                    "switch_connections": {
                        "lan1": {
                            "switch": "switch1",
                            "port": 42.to_string()
                        }
                    }
                }
            ]
    )
}

fn context_api_config(db_file_path: &str, net_ctrl_base_path: &str) -> serde_json::Value {
    json!(
      {
        "db_file_path": db_file_path,
        "net_ctrl_base_path": net_ctrl_base_path,
        "image_api_settings": {
          "max_file_size": "128MiB",
          "store": "/tmp/unused"
        },
        "network_gateway": {
          "ws_gateway_url": "http://example.com/unused"
        },
        "request_timeouts": {
          "single_context_api": 42,
          "context_management_api": 42
        },
        "max_db_connections": 42,
        "context_lifetime": 42,
      }
    )
}

// Check that machine initialization works.
#[tokio::test]
async fn machine_initialization_works() -> Result<(), Box<dyn std::error::Error>> {
    let remote_power_mock = MockServer::start().await;
    let remote_power_address = format!("{}/power", remote_power_mock.uri());

    let remote_usb_mock = MockServer::start().await;
    let remote_usb_address = format!("{}/usb", remote_usb_mock.uri());

    let remote_serial_mock = MockServer::start().await;
    let remote_serial_address = format!("{}/serial", remote_serial_mock.uri());

    let remote_auxiliary_mock = MockServer::start().await;
    let remote_auxiliary_address = format!(
        "{}/auxiliaries/{}",
        remote_auxiliary_mock.uri(),
        AUXILIARY_DEVICE_NAME
    );
    let net_ctrl_mock = MockServer::start().await;

    let net_ctrl_base_address = net_ctrl_mock.uri();

    // Create a temporary file and write machines data to it
    let tmp_file = NamedTempFile::new("machines.json")?;
    let machines_file_content: Value = machines_json(
        remote_usb_address.clone(),
        remote_power_address.clone(),
        remote_auxiliary_address,
        remote_serial_address,
    );
    let machines_file_content = serde_json::to_string(&machines_file_content)?;

    std::fs::write(tmp_file.path(), machines_file_content)?;

    // Setup the mocks to succeed when given the paths described in the remote hands OAS.
    remote_power_mock
        .register(
            Mock::given(any()).respond_with(move |req: &wiremock::Request| {
                assert_eq!(req.method.to_string().to_lowercase(), "post");
                assert_eq!(req.url.path(), "/power/reset");
                ResponseTemplate::new(200)
            }),
        )
        .await;
    remote_usb_mock
        .register(
            Mock::given(any()).respond_with(move |req: &wiremock::Request| {
                assert_eq!(req.method.to_string().to_lowercase(), "post");
                assert_eq!(req.url.path(), "/usb/reset");
                ResponseTemplate::new(200)
            }),
        )
        .await;

    remote_serial_mock
        .register(Mock::given(any()).respond_with(|req: &wiremock::Request| {
            assert_eq!(req.method.to_string().to_lowercase(), "post");
            assert_eq!(req.url.path(), format!("/serial/reset"));
            ResponseTemplate::new(200)
        }))
        .await;

    remote_auxiliary_mock
        .register(Mock::given(any()).respond_with(|req: &wiremock::Request| {
            assert_eq!(
                req.url.path(),
                format!("/auxiliaries/{}/api", AUXILIARY_DEVICE_NAME)
            );
            ResponseTemplate::new(200)
        }))
        .await;

    net_ctrl_mock
        .register(Mock::given(any()).respond_with(ResponseTemplate::new(200)))
        .await;

    let db = TestDb::spawn();
    // Assert that the machine initialization tool succeeds when using a fresh test database.
    let ctx_api_config_tmp_file = NamedTempFile::new("contextapi-config.json")?;

    std::fs::write(
        ctx_api_config_tmp_file.path(),
        serde_json::to_string(&context_api_config(
            db.file
                .as_path()
                .to_str()
                .ok_or_else(|| String::from("invalid db file path"))?,
            &net_ctrl_base_address,
        ))?,
    )?;

    let mut cmd = Command::cargo_bin("machine-ops")?;
    cmd.arg("initialize-machines")
        .arg("run")
        .arg("-m")
        .arg(tmp_file.path())
        .arg("-c")
        .arg(ctx_api_config_tmp_file.path())
        .arg("--machine-reset-timeout")
        .arg("10");

    cmd.assert().success();

    // Let's try to run it again and see that nothing happens if the --skip-free-machines flag is set
    let mut cmd = Command::cargo_bin("machine-ops")?;
    cmd.arg("initialize-machines")
        .arg("run")
        .arg("--machines-file")
        .arg(tmp_file.path())
        .arg("--skip-free-machines")
        .arg("-vvvvv")
        .arg("--context-api-config")
        .arg(ctx_api_config_tmp_file.path())
        .arg("--machine-reset-timeout")
        .arg("0");

    cmd.assert().success();
    Ok(())
}
