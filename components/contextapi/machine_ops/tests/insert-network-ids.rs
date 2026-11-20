// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

mod common;
use common::context_api_config;

use assert_cmd::cargo_bin;
use assert_cmd::prelude::*;
use assert_fs::NamedTempFile;
use db_interaction::schema::network_identifiers as network_identifiers_schema;
use db_interaction::test_utils::TestDb;
use diesel::prelude::*;
use std::collections::HashMap;
use std::process::Command;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::{method, path};

// check that the insert network id command succeeds.
#[tokio::test]
async fn network_id_inserts_work() -> Result<(), Box<dyn std::error::Error>> {
    let mut db = TestDb::spawn();
    let mut cmd = Command::new(cargo_bin!("machine-ops"));

    let net_ids = vec![42];
    // Create a temporary file and write network ids to it
    let tmp_file = NamedTempFile::new("network-ids.json")?;
    let file_content = serde_json::to_string(&net_ids)?;

    std::fs::write(tmp_file.path(), file_content)?;

    let net_ctrl_mock = MockServer::start().await;
    let switch_response = ResponseTemplate::new(200).set_body_json(
        HashMap::<String, String>::from_iter([("switch-a".to_string(), "some-value".to_string())]),
    );
    let mock_switches = Mock::given(method("GET"))
        .and(path("/switches"))
        .respond_with(switch_response);
    net_ctrl_mock.register(mock_switches).await;

    let mock_setup = Mock::given(method("POST"))
        .and(path("/switches/switch-a/setup"))
        .respond_with(ResponseTemplate::new(200));
    net_ctrl_mock.register(mock_setup).await;
    let net_ctrl_base_address = net_ctrl_mock.uri();
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

    cmd.arg("insert-network-ids")
        .arg("run")
        .arg("--network-ids-file")
        .arg(tmp_file.path())
        .arg("--context-api-config")
        .arg(ctx_api_config_tmp_file.path());
    cmd.assert().success();

    let net_ids_from_db: Vec<i16> = network_identifiers_schema::table
        .select(network_identifiers_schema::id)
        .load(&mut db.conn)?;

    assert_eq!(net_ids_from_db, net_ids);
    Ok(())
}
