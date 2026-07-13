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
use axum::extract::rejection::JsonRejection;
use axum::{extract::State, http::StatusCode, Json};
use std::path::PathBuf;
use tracing::{debug, error, instrument};

use crate::{
    app_state::UsbConfigurable,
    usb_config::{UsbConfig, UsbFunctionInfo},
};

/// Router for requests to the /usb API
pub fn usb_router<T: UsbConfigurable>() -> ApiRouter<T> {
    ApiRouter::new()
        .api_route_with(
            "/usb",
            get_with(handle_get_usb::<T>, handle_get_usb_doc)
                .put_with(handle_put_usb::<T>, handle_put_usb_doc)
                .delete_with(handle_delete_usb::<T>, handle_delete_usb_doc),
            usb_api_doc,
        )
        .api_route_with(
            "/usb/reset",
            post_with(handle_reset::<T>, handle_reset_doc),
            usb_api_doc,
        )
}

/// The tag for the UsbAPI. Used for (un-)folding this API in the UI.
fn usb_api_doc(op: TransformPathItem) -> TransformPathItem {
    op.tag("USB API")
}

/// GET handler. Returns the currently active USB configuration.
#[instrument(skip(state))]
async fn handle_get_usb<T: UsbConfigurable>(
    State(state): State<T>,
) -> Result<Json<Vec<UsbFunctionInfo>>, (StatusCode, String)> {
    // Get information per function if available.
    // No functions configured does not warrant an error, simply return empty vector.
    let mut infos: Vec<_> = state.get_function_infos().await.unwrap_or_default();
    // Loop over function information and set path to be the filename whenever we
    // encounter the storage variant.
    for info in &mut infos {
        if let UsbFunctionInfo::Storage { ref mut luns } = info {
            for lun in luns {
                lun.path = PathBuf::from(&lun.path)
                    .file_name()
                    .ok_or((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("could not parse lun path: '{0}'", lun.path).to_string(),
                    ))?
                    .to_string_lossy()
                    .to_string();
            }
        }
    }
    Ok(Json(infos))
}

/// Documentation for GET handler.
/// Only expected error is `StatusCode::INTERNAL_SERVER_ERROR` on issues when
/// trying to read internal USB functions state.
fn handle_get_usb_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Query USB interface")
        .description("Query the currently setup USB functions")
        .response::<200, Json<Vec<UsbFunctionInfo>>>()
        .response_with::<500, &str, _>(|op| {
            op.description("Could not read internal USB functions state")
        })
}

/// PUT handler. Configure and activate USB with given config.
#[instrument(skip_all)]
async fn handle_put_usb<T: UsbConfigurable>(
    State(state): State<T>,
    usb_config: Result<Json<UsbConfig>, JsonRejection>,
) -> Result<Json<Vec<UsbFunctionInfo>>, (StatusCode, String)> {
    let usb_config = match usb_config {
        Ok(Json(conf)) => conf,
        Err(r) => {
            error!(json_error = ?r, "UsbConfig parsing failed");
            return Err((
                StatusCode::BAD_REQUEST,
                "Unable to parse usb configuration".to_string(),
            ));
        }
    };

    // Handing over an empty function list is the same as calling DELETE
    if usb_config.functions.is_empty() {
        debug!("deconfiguring USB");
        state.deconfigure().await.map_err(|e| {
            error!(error = %e, "error deconfiguring USB OTG functions");
            (StatusCode::INTERNAL_SERVER_ERROR, format!("error: {e}"))
        })?;

        debug!("USB deconfigured");
        Ok(Json(vec![]))
    } else {
        debug!("configuring USB");
        state.configure(usb_config).await.map_err(|e| {
            error!(error = %e, "error configuring USB OTG functions");
            (StatusCode::BAD_REQUEST, format!("error: {e}"))
        })?;

        debug!("USB configured");
        // return info on success
        handle_get_usb(State(state)).await
    }
}

/// Documentation for PUT handler.
/// Expected errors are `StatusCode::BAD_REQUEST` and `StatusCode::INTERNAL_SERVER_ERROR`
/// on issues with reading the internal USB state or deconfiguring USB on empty list.
fn handle_put_usb_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Setup USB interface")
        .description("Configure USB functions and activates them. Returns the currently setup USB functions.")
        .response::<200, Json<Vec<UsbFunctionInfo>>>()
        .response_with::<400, &str, _>(|op| op.description("Bad Request"))
        .response_with::<500, &str, _>(|op| op.description("Could not read internal USB functions state or configure empty list"))
}

/// DELETE handler. Deconfigure current USB config.
#[instrument(skip(state))]
async fn handle_delete_usb<T: UsbConfigurable>(
    State(state): State<T>,
) -> Result<(), (StatusCode, String)> {
    debug!("deconfiguring USB");
    state.deconfigure().await.map_err(|e| {
        error!(error = %e, "error deconfiguring USB OTG functions");
        (StatusCode::INTERNAL_SERVER_ERROR, format!("error: {e}"))
    })?;

    debug!("USB deconfigured");

    Ok(())
}

/// Documentation for DELETE handler.
/// Only expected error is `StatusCode::INTERNAL_SERVER_ERROR` on deconfiguring
/// USB.
fn handle_delete_usb_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Deconfigure USB interface")
        .description("Deactivate the configured USB functions.")
        .response::<200, ()>()
        .response_with::<500, &str, _>(|op| op.description("Could not deconfigure USB functions"))
}

/// POST handler to reset configured USB config for context termination.
#[instrument(skip(state))]
pub async fn handle_reset<T: UsbConfigurable>(
    State(state): State<T>,
) -> Result<(), (StatusCode, String)> {
    handle_delete_usb(State(state)).await
}

/// Documentation for reset handler.
/// Only expected error is `StatusCode::INTERNAL_SERVER_ERROR` on deconfiguring
/// USB.
pub fn handle_reset_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Reset USB interface")
        .description("Reset the USB interface")
        .response::<200, ()>()
        .response_with::<500, &str, _>(|op| op.description("Could not deconfigure USB functions"))
}
