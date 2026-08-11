// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, JsonSchema, Eq, PartialEq, Hash, Clone)]
pub struct VlanPort {
    /// The unique ID of the VLAN.
    pub vlan_id: u16,

    /// The unique ID of the Port.
    pub port_id: String,

    /// The Ports mode.
    pub port_mode: PortMode,
}

#[derive(Deserialize, Serialize, JsonSchema, Eq, PartialEq, Hash, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PortMode {
    Untagged,
    TaggedStatic,
    Forbidden,
}
