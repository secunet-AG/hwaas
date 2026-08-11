// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, JsonSchema, Clone)]
pub struct PortElementList {
    pub port_element: Vec<Port>,
}

#[derive(Deserialize, Serialize, JsonSchema, Eq, PartialEq, Hash, Clone)]
#[serde(default)]
pub struct Port {
    /// The unique ID of the Port.
    pub id: String,

    /// The name of the Port. An \"\" denotes removal of already configured name if exists.
    pub name: String,

    /// Whether the Port is enabled.
    pub is_port_enabled: bool,

    /// Specifies Whether the  Link status is up or down.
    pub is_port_up: bool,

    /// The Port Config Mode.
    pub config_mode: PortConfigMode,
    // some fileds are skipped:
    // (trunk_mode, lacp_status, trunk_group,
    //  is_flow_control_enabled, is_dsnoop_port_trusted)
}

impl Default for Port {
    fn default() -> Self {
        Self {
            id: "".to_string(),
            name: "".to_string(),
            is_port_enabled: true,
            is_port_up: true,
            config_mode: PortConfigMode::PcmAuto,
        }
    }
}

#[derive(Deserialize, Serialize, JsonSchema, Eq, PartialEq, Hash, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PortConfigMode {
    PcmAuto,
    // 14 further elements are skipped
}
