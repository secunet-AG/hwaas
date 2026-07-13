// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::app_state::AppState;
use aruba_structs::port::{Port, PortElementList};
use axum::extract::{Path, State};
use axum::Json;
use std::sync::Arc;
use tracing::info;

pub(crate) async fn ports(State(state): State<Arc<AppState>>) -> Json<PortElementList> {
    Json(PortElementList {
        port_element: state.ports.clone().into_iter().collect(),
    })
}

pub(crate) async fn enable_port(Path(port_id): Path<String>, Json(port): Json<Port>) -> Json<Port> {
    info!("change port: {}", port_id);
    Json(port)
}
