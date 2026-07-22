// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use aide::transform::TransformOperation;
use axum::{extract::State, http::StatusCode, Json};

use crate::api::HasSerial;

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
        .response::<200, ()>()
}

/// Handler for POST /serial/reset requests.
/// Loops over all known serial devices and clears the buffer.
pub async fn handle_reset<S: HasSerial + Send + Sync>(
    State(state): State<S>,
) -> Result<(), (StatusCode, String)> {
    for serial in state.get_serials().await {
        serial.clear_buffer();
    }
    Ok(())
}

/// Documentation for reset handler.
pub fn handle_reset_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Reset all serial interfaces")
        .description("Reset all serial interfaces")
        .response::<200, ()>()
}
