// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::api::HasSerial;
use aide::transform::TransformOperation;
use axum::response::{IntoResponse, Response};
use axum::{extract::State, http::StatusCode, Json};

/// Handler for GET /serial requests. Returns all known serial ids.
pub async fn handle_get_all<S: HasSerial + Send + Sync>(
    State(state): State<S>,
) -> Result<Json<Vec<String>>, (StatusCode, String)> {
    let serial_ids = state.get_serial_ids().await;
    Ok(Json(serial_ids))
}

/// Documentation for GET all handler.
pub fn handle_get_all_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Query serial interfaces")
        .description("Query for the ids of all configured serial interfaces")
        .response_with::<{ StatusCode::OK.as_u16() }, Json<Vec<String>>, _>(|op| {
            op.description("list of all serial interfaces")
        })
}

/// Handler for POST /serial/reset requests.
/// Loops over all known serial devices and clears the buffer.
pub async fn handle_reset<S: HasSerial + Send + Sync>(State(state): State<S>) -> Response {
    for serial in state.get_serials().await {
        let mut ring = serial.ring.write().await;
        ring.clear();
    }
    StatusCode::NO_CONTENT.into_response()
}

/// Documentation for reset handler.
pub fn handle_reset_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Reset all serial interfaces")
        .description("Reset all serial interfaces")
        .response::<{ StatusCode::NO_CONTENT.as_u16() }, ()>()
}
