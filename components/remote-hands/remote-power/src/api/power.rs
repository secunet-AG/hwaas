// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use aide::transform::TransformOperation;
use axum::{http::StatusCode, Json};
use tracing::error;

use super::{LockedControl, PowerInterface};
use crate::{
    api::power_status::PowerStatus,
    power::{PowerControl, PowerState},
};

/// GET handler to return current `PowerStatus` for one interface with `power_id`.
/// Returning `PowerStatus::On` on `PowerState::Reset`, since "reset" is just a
/// temporary state and will switch to "on" soon enough.
/// Return error on `PowerState::Unknown`.
pub async fn handle_get_power_interface(
    LockedControl(PowerInterface {
        power_id,
        mut power_backend,
    }): LockedControl,
) -> Result<Json<PowerStatus>, (StatusCode, String)> {
    let state = power_backend.query().await.map_err(|e| {
        error!("{}", format!("power_backend query error: {e}"));
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("power_backend query error: {e}"),
        )
    })?;
    match state {
        PowerState::Off => Ok(Json(PowerStatus::off(power_id))),
        PowerState::On => Ok(Json(PowerStatus::on(power_id))),
        PowerState::Reset => Ok(Json(PowerStatus::on(power_id))),
        PowerState::Unknown => {
            error!("power script error: unknown power state");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "power script error: unknown power state".to_string(),
            ))
        }
    }
}

/// Documentation for GET handler for one interface.
/// Only expected error is `StatusCode::INTERNAL_SERVER_ERROR` on power script
/// errors.
pub fn handle_get_power_interface_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Query power interface")
        .description("Query for power status of single power interface")
        .response::<200, Json<PowerStatus>>()
        .response_with::<500, &str, _>(|op| op.description("Power script error"))
}

/// PUT handler to power on machine and return current `PowerStatus` for one
/// interface with `power_id`.
pub async fn handle_put_power_interface(
    LockedControl(PowerInterface {
        power_id,
        mut power_backend,
    }): LockedControl,
) -> Result<Json<PowerStatus>, (StatusCode, String)> {
    power_backend.power_on().await.map_err(|e| {
        error!("{}", format!("power_backend power_on error: {e}"));
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("power_backend power_on error: {e}"),
        )
    })?;

    handle_get_power_interface(LockedControl(PowerInterface {
        power_id,
        power_backend,
    }))
    .await
}

/// Documentation for PUT handler for one interface.
/// Only expected error is `StatusCode::INTERNAL_SERVER_ERROR` on power script
/// errors.
pub fn handle_put_power_interface_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Power on power interface")
        .description("Power on single power interface")
        .response::<200, Json<PowerStatus>>()
        .response_with::<500, &str, _>(|op| op.description("Power script error"))
}

/// DELETE handler to power off machine and return current `PowerStatus` for one
/// interface with `power_id`.
pub async fn handle_delete_power_interface(
    LockedControl(PowerInterface {
        power_id,
        mut power_backend,
    }): LockedControl,
) -> Result<Json<PowerStatus>, (StatusCode, String)> {
    power_backend.power_off().await.map_err(|e| {
        error!("{}", format!("power_backend power_off error: {e}"));
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("power_backend power_off error: {e}"),
        )
    })?;

    handle_get_power_interface(LockedControl(PowerInterface {
        power_id,
        power_backend,
    }))
    .await
}

/// Documentation for DELETE handler for one interface.
/// Only expected error is `StatusCode::INTERNAL_SERVER_ERROR` on power script
/// errors.
pub fn handle_delete_power_interface_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Power off power interface")
        .description("Power off single power interface")
        .response::<200, Json<PowerStatus>>()
        .response_with::<500, &str, _>(|op| op.description("Power script error"))
}
