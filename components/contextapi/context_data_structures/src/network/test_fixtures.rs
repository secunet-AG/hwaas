// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

const SINGLE_MACHINE_SINGLE_INTERFACE: &str = r#" {
    "machine1": {
        "LAN1": {}
    }
}"#;

const SINGLE_MACHINE_MULTIPLE_INTERFACES: &str = r#"
{
    "machine1": {
        "LAN1": {},
        "LAN2": {}
    }
}"#;

const MULTIPLE_MACHINES_SINGLE_INTERFACE: &str = r#"
{
    "machine1": {
        "LAN1": {}
    },
    "machine2": {
        "LAN1": {}
    }
}"#;

const MULTIPLE_MACHINES_MULTIPLE_INTERFACES: &str = r#"
{
    "machine1": {
        "LAN1": {},
        "LAN2": {}
    },
    "machine2": {
        "LAN1": {}
    },
    "machine3": {
        "LAN1": {},
        "LAN2": {},
        "LAN3": {}
    }
}"#;

pub const VALID_JSON_REPRESENTATIONS: [&str; 4] = [
    SINGLE_MACHINE_SINGLE_INTERFACE,
    SINGLE_MACHINE_MULTIPLE_INTERFACES,
    MULTIPLE_MACHINES_SINGLE_INTERFACE,
    MULTIPLE_MACHINES_MULTIPLE_INTERFACES,
];
