// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use aide::{
    axum::{ApiRouter, routing::get_with},
    transform::{TransformOperation, TransformPathItem},
};
use axum::{Json, http::StatusCode};

use super::ExtractAux;
use crate::api::activation_info::AuxiliaryDevice;
use crate::{app_state::AppState, auxiliary::AuxState};

/// Router for requests to a single auxiliary device
pub fn aux_device_router() -> ApiRouter<AppState> {
    ApiRouter::new().api_route_with(
        "/auxiliaries/:device_id",
        get_with(handle_get_aux, handle_get_aux_doc),
        aux_api_doc,
    )
}

/// Router for requests to the activation API of single auxiliary devices
pub fn aux_device_activation_router() -> ApiRouter<AppState> {
    ApiRouter::new().api_route_with(
        "/auxiliaries/:device_id/activation",
        get_with(handle_get_aux_activation, handle_get_aux_activation_doc)
            .put_with(handle_put_aux_activation, handle_put_aux_activation_doc),
        aux_api_doc,
    )
}

/// The tag for the AuxiliaryAPI. Used for (un-)folding this API in the UI.
fn aux_api_doc(op: TransformPathItem) -> TransformPathItem {
    op.tag("Auxiliary Device API")
}

/// GET handler to return extracted `AuxiliaryDevice` with state information.
async fn handle_get_aux(
    ExtractAux { state: inner, .. }: ExtractAux,
) -> Result<Json<AuxiliaryDevice>, Json<String>> {
    let state = inner.query().await.map_err(|e| format!("error: {e}"))?;
    match state {
        AuxState::Off => Ok(Json(AuxiliaryDevice::off(&inner.aux_config.id))),
        AuxState::On => Ok(Json(AuxiliaryDevice::on(&inner.aux_config.id))),
    }
}

/// Documentation for GET handler for single auxiliary device.
/// Only expected error is `StatusCode::NOT_FOUND` on unknown ID.
fn handle_get_aux_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Query information")
        .description("Query for auxiliary device information.")
        .response_with::<200, Json<AuxiliaryDevice>, _>(|op| {
            op.description("JSON representing auxiliary device information for given device ID.")
                .example(AuxiliaryDevice::on("example-device"))
        })
        .response_with::<404, &str, _>(|op| op.description("Auxiliary Device ID is unknown."))
}

/// GET handler to return activation state of extracted `AuxiliaryDevice`.
async fn handle_get_aux_activation(
    ExtractAux { state: inner, .. }: ExtractAux,
) -> Result<Json<bool>, Json<String>> {
    let state = inner.query().await.map_err(|e| format!("error: {e}"))?;
    match state {
        AuxState::Off => Ok(Json(false)),
        AuxState::On => Ok(Json(true)),
    }
}

/// Documentation for GET handler for activation state of single auxiliary device.
/// Only expected error is `StatusCode::NOT_FOUND` on unknown device ID.
fn handle_get_aux_activation_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Query activation status")
        .description("Query for auxiliary device activation status.")
        .response_with::<200, Json<bool>, _>(|op| {
            op.description(
                "Boolean representing auxiliary device activation information for given device ID.",
            )
        })
        .response_with::<404, &str, _>(|op| op.description("Auxiliary Device ID is unknown."))
}

/// PUT handler to switch activation state of extracted `AuxiliaryDevice`.
async fn handle_put_aux_activation(
    ExtractAux { state: inner, .. }: ExtractAux,
    Json(desired_state): Json<bool>,
) -> Result<(), (StatusCode, String)> {
    let current_state = inner
        .query()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("error: {e}")))?;
    let result = !current_state.match_bool(desired_state);
    if result {
        match desired_state {
            true => inner
                .power_on()
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("error: {e}")))?,
            false => inner
                .power_off()
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("error: {e}")))?,
        };
    };
    Ok(())
}

/// Documentation for PUT handler for activation state of single auxiliary device.
/// Expected errors are
/// `StatusCode::BAD_REQUEST` on invalid input,
/// `StatusCode::NOT_FOUND` on unknown device ID and
/// `StatusCode::INTERNAL_SERVER_ERROR` on activation command failure.
fn handle_put_aux_activation_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Set the activation state")
        .description(
            "Set the activation state to the specified value. 'true' activates the device.",
        )
        .response::<200, ()>()
        .response_with::<400, &str, _>(|op| op.description("Invalid body."))
        .response_with::<404, &str, _>(|op| op.description("Auxiliary Device ID is unknown."))
        .response_with::<500, &str, _>(|op| {
            op.description("Auxiliary Device activation command failed.")
        })
}
