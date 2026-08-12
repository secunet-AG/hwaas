// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::network_type_ids::{PortID, SwitchID};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

mod external_api_errors;
pub(crate) mod get_switch_info;
pub(crate) mod get_switches;
pub(crate) mod handle_ports;
pub(crate) mod setup_data;
pub(crate) mod setup_switch;

#[derive(Deserialize, Serialize, Debug, JsonSchema)]
pub struct PathParamsSwitchID {
    /// Context access token
    pub switch_id: SwitchID,
}

#[derive(Deserialize, Serialize, Debug, JsonSchema)]
pub struct PathParamsSwitchAndPortID {
    /// ID of a switch
    pub switch_id: SwitchID,

    /// PortID of the switch
    pub port_id: PortID,
}
