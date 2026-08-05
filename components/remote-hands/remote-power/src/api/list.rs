// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::{
    AppState,
    api::power_status::PowerStatus,
    power::{PowerControl, PowerState},
};
use aide::transform::TransformOperation;
use axum::{Json, extract::State, http::StatusCode};
use std::collections::HashMap;
use tracing::error;

/// GET handler to return current `PowerStatus` list for all interfaces.
/// Returning `PowerStatus::On` on `PowerState::Reset`, since "reset" is just a
/// temporary state and will switch to "on" soon enough.
/// Return error on `PowerState::Unknown` instead of list.
pub async fn handle_get_power(
    State(state): State<AppState>,
) -> Result<Json<Vec<PowerStatus>>, (StatusCode, String)> {
    let power_info = HashMap::clone(&state.controls).into_iter();
    let mut v = Vec::new();
    for (power_id, power_backend) in power_info {
        let mut power_backend = power_backend.lock().await;
        let state = power_backend.query().await.map_err(|e| {
            error!(
                "{}",
                format!("power_backend query error for {power_id}: {e}")
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("power_backend query error for {power_id}: {e}"),
            )
        })?;
        let status = match state {
            PowerState::Off => PowerStatus::off(power_id),
            PowerState::On => PowerStatus::on(power_id),
            PowerState::Reset => PowerStatus::on(power_id),
            PowerState::Unknown => {
                error!("power script error for {power_id}: unknown power state");
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "power script error for {power_id}: unknown power state".to_string(),
                ));
            }
        };
        v.push(status)
    }
    Ok(Json(v))
}

/// Documentation for GET handler for all interfaces.
/// Only expected error is `StatusCode::INTERNAL_SERVER_ERROR` on power script
/// errors.
pub fn handle_get_power_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Query all power interfaces")
        .description("Query for power status of all power interfaces")
        .response::<200, Json<Vec<PowerStatus>>>()
        .response_with::<500, &str, _>(|op| op.description("Power script error"))
}

/// PUT handler to power on all interfaces of a machine and return the
/// `PowerStatus` for all these interfaces.
pub async fn handle_put_power(
    State(state): State<AppState>,
) -> Result<Json<Vec<PowerStatus>>, (StatusCode, String)> {
    let power_info = HashMap::clone(&state.controls).into_iter();
    for (power_id, power_backend) in power_info {
        let mut power_backend = power_backend.lock().await;
        power_backend.power_on().await.map_err(|e| {
            error!(
                "{}",
                format!("power_backend power_on error for {power_id}: {e}")
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("power_backend power_on error for {power_id}: {e}"),
            )
        })?;
    }

    handle_get_power(State(state)).await
}

/// Documentation for PUT handler for all interfaces.
/// Only expected error is `StatusCode::INTERNAL_SERVER_ERROR` on power script
/// errors.
pub fn handle_put_power_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Power on all power interfaces")
        .description("Power on all power interfaces")
        .response::<200, Json<Vec<PowerStatus>>>()
        .response_with::<500, &str, _>(|op| op.description("Power script error"))
}

/// DELETE handler to power off all interfaces of a machine and return the
/// `PowerStatus` for all these interfaces.
pub async fn handle_delete_power(
    State(state): State<AppState>,
) -> Result<Json<Vec<PowerStatus>>, (StatusCode, String)> {
    let power_info = HashMap::clone(&state.controls).into_iter();
    for (power_id, power_backend) in power_info {
        let mut power_backend = power_backend.lock().await;
        power_backend.power_off().await.map_err(|e| {
            error!(
                "{}",
                format!("power_backend power_on error for {power_id}: {e}")
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("power_backend power_on error for {power_id}: {e}"),
            )
        })?;
    }

    handle_get_power(State(state)).await
}

/// Documentation for DELETE handler for all interfaces.
/// Only expected error is `StatusCode::INTERNAL_SERVER_ERROR` on power script
/// errors.
pub fn handle_delete_power_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Power off all power interfaces")
        .description("Power off all power interfaces")
        .response::<200, Json<Vec<PowerStatus>>>()
        .response_with::<500, &str, _>(|op| op.description("Power script error"))
}

/// Reset handler for context termination.
/// Currently only call the DELETE ALL handler, might change in the future.
pub async fn handle_reset(
    State(state): State<AppState>,
) -> Result<Json<Vec<PowerStatus>>, (StatusCode, String)> {
    handle_delete_power(State(state)).await
}

/// Documentation for the reset handler.
/// Only expected error is `StatusCode::INTERNAL_SERVER_ERROR` on power script
/// errors.
pub fn handle_reset_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Reset all power interfaces")
        .description("Reset all power interfaces")
        .response::<200, Json<Vec<PowerStatus>>>()
        .response_with::<500, &str, _>(|op| op.description("Power script error"))
}
