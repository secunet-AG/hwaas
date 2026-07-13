// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::{PortID, VlanID};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::net::IpAddr;

#[derive(Eq, Hash, Clone, PartialEq, Debug, JsonSchema)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct SwitchDetails {
    pub ip: IpAddr,
    pub port: Option<u16>,
    #[serde(default)]
    pub credentials: Option<Credentials>,
    pub critical_ports: CriticalPorts,
    pub default_vlan: VlanID,
    pub mgmt_vlan: VlanID,
}

#[derive(Eq, Hash, Clone, PartialEq, Debug, JsonSchema)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct CriticalPorts {
    pub mgmt_ports: Vec<PortID>,
    pub trunk_ports: Vec<PortID>,
}

#[derive(Eq, Hash, Clone, PartialEq, Debug, JsonSchema)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

impl SwitchDetails {
    pub fn new(
        ip: IpAddr,
        credentials: Option<Credentials>,
        critical_ports: CriticalPorts,
        default_vlan: VlanID,
        mgmt_vlan: VlanID,
    ) -> Self {
        SwitchDetails {
            ip,
            port: None,
            credentials,
            critical_ports,
            default_vlan,
            mgmt_vlan,
        }
    }

    pub fn get(&self) -> &Self {
        self
    }

    // TODO: rm those and use `Self::get()`
    pub fn get_ip(&self) -> &IpAddr {
        self.ip.borrow()
    }

    pub fn get_credentials(&self) -> &Option<Credentials> {
        self.credentials.borrow()
    }

    pub fn get_critical_ports(&self) -> &CriticalPorts {
        self.critical_ports.borrow()
    }
}
