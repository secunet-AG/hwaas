// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::app_state::AppState;

use aruba_structs::vlan_port::{PortMode, VlanPort};
use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct VlanPortList {
    vlan_port_element: Vec<VlanPort>,
}
pub(crate) async fn vlan_ports(State(state): State<Arc<AppState>>) -> Json<VlanPortList> {
    info!("Returning VlanPortList");

    let ports = state
        .ports
        .iter()
        .map(|p| VlanPort {
            vlan_id: 1,
            port_id: p.id.to_string(),
            port_mode: PortMode::PomUntagged,
        })
        .collect();

    Json(VlanPortList {
        vlan_port_element: ports,
    })
}
