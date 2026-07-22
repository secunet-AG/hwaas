// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use assert_cmd::prelude::*;
use std::process::Command;

// check that the command succeeds and that stdout contains some things we expect to be in the JSON schema
// such as "array".
#[test]
fn schema_generation_works() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("machine-ops")?;
    cmd.arg("insert-network-ids").arg("print-schema");
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("array"));
    Ok(())
}
