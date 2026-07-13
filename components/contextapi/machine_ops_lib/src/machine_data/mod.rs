// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use axum::http::{uri::InvalidUri, Uri};
use db_interaction::models::{aliases::MachineId, machines::SwitchPortValue};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

mod remote_base_urls;
pub use remote_base_urls::*;

pub type MachineNetworkInterface = String;

pub type MachineSerialDevice = String;

pub type MachineAuxDevice = String;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MachineData {
    /// The identifier of the machine.
    pub id: MachineId,
    /// The machine's platform.
    pub platform: String,
    /// An address for the machine's remote-usb server.
    pub remote_usb: RemoteUsbBaseUrl,
    /// An address for the machine's remote-power server.
    pub remote_power: RemotePowerBaseUrl,
    /// An address for the machine's remote-serial server.
    #[serde(default)]
    pub remote_serial: Option<RemoteSerialBaseUrl>,
    /// The machine's switch connections
    pub switch_connections: HashMap<MachineNetworkInterface, SwitchPortValue>,
    /// An address for the machine's remote auxiliary server.
    #[serde(default)]
    pub remote_auxiliary: Option<RemoteAuxiliaryBaseUrl>,
}
