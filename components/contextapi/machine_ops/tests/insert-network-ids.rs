// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use assert_cmd::prelude::*;
use assert_fs::NamedTempFile;
use db_interaction::schema::network_identifiers as network_identifiers_schema;
use db_interaction::test_utils::TestDb;
use diesel::prelude::*;
use std::process::Command;

// check that the insert network id command succeeds.
#[test]
fn network_id_inserts_work() -> Result<(), Box<dyn std::error::Error>> {
    let mut db = TestDb::spawn();
    let mut cmd = Command::cargo_bin("machine-ops")?;

    let net_ids = vec![42];
    // Create a temporary file and write network ids to it
    let tmp_file = NamedTempFile::new("network-ids.json")?;
    let file_content = serde_json::to_string(&net_ids)?;

    std::fs::write(tmp_file.path(), file_content)?;

    cmd.arg("insert-network-ids")
        .arg("run")
        .arg("--network-ids-file")
        .arg(tmp_file.path())
        .arg("--database")
        .arg(db.file.as_path());
    cmd.assert().success();

    let net_ids_from_db: Vec<i16> = network_identifiers_schema::table
        .select(network_identifiers_schema::id)
        .load(&mut db.conn)?;

    assert_eq!(net_ids_from_db, net_ids);
    Ok(())
}
