// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

//! JSON data returned by Aruba Switch

use crate::network_type_ids::{PortID, VlanID};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Default, Debug)]
pub(crate) enum ArubaVlanPortMode {
    #[default]
    #[serde(rename = "POM_UNTAGGED")]
    Untagged,

    #[serde(rename = "POM_TAGGED_STATIC")]
    Tagged,

    #[serde(rename = "POM_FORBIDDEN")]
    Forbidden,
}

#[derive(Deserialize, Serialize, Debug)]
pub(crate) struct ArubaVlanPort {
    /// The unique ID of the VLAN.
    #[serde(flatten)]
    pub(crate) vlan_id: VlanID,

    /// The unique ID of the Port.
    pub(crate) port_id: String,

    /// The Ports mode.
    pub(crate) port_mode: ArubaVlanPortMode,
}

impl From<(&PortID, &VlanID)> for ArubaVlanPort {
    fn from((port_id, vlan_id): (&PortID, &VlanID)) -> Self {
        Self {
            vlan_id: vlan_id.clone(),
            port_id: port_id.to_string(),
            port_mode: ArubaVlanPortMode::Untagged,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub(crate) struct ArubaVlanPortElementList {
    pub(crate) vlan_port_element: Vec<ArubaVlanPort>,
}
