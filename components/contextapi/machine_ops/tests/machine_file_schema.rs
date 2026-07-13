// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use assert_cmd::cargo_bin;
use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

// check that the command succeeds and that stdout contains some things we expect to be in the JSON schema
// such as "machine", "id", "platform" and "switch_connections".
#[test]
fn schema_generation_works() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::new(cargo_bin!("machine-ops"));
    cmd.arg("initialize-machines").arg("print-schema");
    let predicate = predicate::str::contains("machine")
        .and(predicate::str::contains("id"))
        .and(predicate::str::contains("platform"))
        .and(predicate::str::contains("switch_connections"))
        .and(predicate::str::contains("remote_serial"));
    cmd.assert().success().stdout(predicate);
    Ok(())
}
