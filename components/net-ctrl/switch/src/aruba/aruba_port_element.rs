// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

//! JSON data returned by Aruba Switch

use network_type_ids::PortID;
use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct ArubaPort {
    pub(crate) id: String,
}

#[derive(Deserialize)]
pub(crate) struct ArubaPortElementList {
    pub(crate) port_element: Vec<ArubaPort>,
}

impl From<ArubaPort> for PortID {
    fn from(p: ArubaPort) -> Self {
        PortID::new(p.id)
    }
}
