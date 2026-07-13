// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use aide::{
    axum::{
        routing::{get_with, post_with},
        ApiRouter,
    },
    transform::{TransformOperation, TransformPathItem},
};
use axum::{extract::State, http::StatusCode, Json};
use std::collections::HashMap;

use crate::api::activation_info::AuxiliaryDevice;
use crate::app_state::AppState;

/// Router for requests to all auxiliary devices
pub fn all_aux_router() -> ApiRouter<AppState> {
    ApiRouter::new()
        .api_route_with(
            "/auxiliaries",
            get_with(handle_get_all_aux, handle_get_all_aux_doc),
            aux_api_doc,
        )
        .api_route_with(
            "/auxiliaries/reset",
            post_with(handle_reset, handle_reset_doc),
            aux_api_doc,
        )
}

/// The tag for the AuxiliaryAPI. Used for (un-)folding this API in the UI.
fn aux_api_doc(op: TransformPathItem) -> TransformPathItem {
    op.tag("Auxiliary Device API")
}

/// GET handler to retrieve information about all configured auxiliary devices.
async fn handle_get_all_aux(
    State(state): State<AppState>,
) -> Result<Json<Vec<AuxiliaryDevice>>, (StatusCode, String)> {
    let infos = state.get_aux_infos().await.ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "Internal state error".to_string(),
    ))?;

    Ok(Json(infos))
}
/// Documentation for GET handler to retrieve information about all devices.
/// Only expected error is `StatusCode::INTERNAL_SERVER_ERROR` on internal state.
fn handle_get_all_aux_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Query all auxiliary devices")
        .description("Query for information of all connected auxiliary devices")
        .response_with::<200, Json<Vec<AuxiliaryDevice>>, _>(|op| {
            op.description(
                "JSON representing auxiliary device information for all configured devices.",
            )
            .example(vec![
                AuxiliaryDevice::on("example-device"),
                AuxiliaryDevice::off("example-device-2"),
            ])
        })
        .response_with::<500, &str, _>(|op| op.description("Auxiliary device state error"))
}

/// POST handler to reset all configured auxiliary devices for context termination.
/// Deactivates all auxiliary devices and returns a list of `AuxiliaryDevice`s.
async fn handle_reset(
    State(state): State<AppState>,
) -> Result<Json<Vec<AuxiliaryDevice>>, (StatusCode, String)> {
    let aux_devices = HashMap::clone(&state.aux_devices).into_iter();
    for (aux, inner) in aux_devices {
        inner.power_off().await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("deactivation for '{aux}' failed: {e}"),
            )
        })?;
    }

    handle_get_all_aux(State(state)).await
}

/// Documentation for the reset handler.
/// Only expected error is `StatusCode::INTERNAL_SERVER_ERROR` on internal state
/// or activation command issues.
fn handle_reset_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Reset all auxiliary devices")
        .description("Reset all auxiliary devices")
        .response::<200, Json<Vec<AuxiliaryDevice>>>()
        .response_with::<500, &str, _>(|op| {
            op.description("Auxiliary device state or script error")
        })
}
