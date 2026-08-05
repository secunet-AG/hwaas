// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

mod list;
mod power;
pub mod power_status;

use crate::{AppState, power::PowerControlBackend};
use aide::{
    OperationIo,
    axum::routing::{get_with, post_with},
    openapi::OpenApi,
    transform::TransformPathItem,
};
use axum::{
    RequestPartsExt, Router, async_trait,
    extract::{FromRef, FromRequestParts, Path},
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use schemars::JsonSchema;
use serde::Deserialize;
use std::convert::Infallible;
use tokio::sync::OwnedMutexGuard;

use self::list::*;
use self::power::*;

/// Create the router for the `PowerAPI`
pub async fn get_router<S>(state: AppState) -> Result<Router<S>, Infallible> {
    Ok(prepare_api_router(state).await?.0)
}

/// Build a `OpenAPI` based on the router implementation
pub async fn get_api<S>(state: AppState) -> Result<OpenApi, Infallible> {
    Ok(prepare_api_router::<S>(state).await?.1)
}

/// Prepare router with all sub routes.
pub async fn prepare_api_router<S>(state: AppState) -> Result<(Router<S>, OpenApi), Infallible> {
    let (router, api) = remote_axum::api_router(
        "remote-hands power service",
        env!("CARGO_PKG_VERSION"),
        |router| async {
            Ok::<_, Infallible>(
                router
                    .api_route_with(
                        "/power",
                        get_with(handle_get_power, handle_get_power_doc)
                            .put_with(handle_put_power, handle_put_power_doc)
                            .delete_with(handle_delete_power, handle_delete_power_doc),
                        power_api_doc,
                    )
                    .api_route_with(
                        "/power/reset",
                        post_with(handle_reset, handle_reset_doc),
                        power_api_doc,
                    )
                    .api_route_with(
                        "/power/:power_interface",
                        get_with(handle_get_power_interface, handle_get_power_interface_doc)
                            .put_with(handle_put_power_interface, handle_put_power_interface_doc)
                            .delete_with(
                                handle_delete_power_interface,
                                handle_delete_power_interface_doc,
                            ),
                        power_api_doc,
                    ),
            )
        },
    )
    .await?;
    let router = router.with_state(state);
    Ok((router, api))
}

/// The tag for the PowerAPI. Used for (un-)folding this API in the UI.
fn power_api_doc(op: TransformPathItem) -> TransformPathItem {
    op.tag("Power API")
}

#[derive(Clone, Deserialize, JsonSchema)]
/// The ID in the API to specify the name of the power interface to use.
pub struct ControlID {
    power_interface: String,
}

#[derive(OperationIo)]
#[aide(input_with = "Path<ControlID>")]
/// Struct with the power backend retrieved by the given `ControlID`.
/// Locked since we don't want to execute two commands in parallel.
pub struct LockedControl(PowerInterface);

/// Internal type with the power interface id/name and the backend, which is
/// encapsulated in an `OwnedMutexGuard` for safety reasons.
pub struct PowerInterface {
    pub power_id: String,
    pub power_backend: OwnedMutexGuard<PowerControlBackend>,
}

#[async_trait]
impl<S> FromRequestParts<S> for LockedControl
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    /// Split the parts of the request and create structs as needed in the API handling.
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path(ControlID {
            power_interface: control_id,
        }) = parts.extract().await.map_err(IntoResponse::into_response)?;
        if let Some(mutex) = AppState::from_ref(state).controls.get(&control_id) {
            let guard = mutex.clone().lock_owned().await;
            Ok(LockedControl(PowerInterface {
                power_id: control_id,
                power_backend: guard,
            }))
        } else {
            Err(StatusCode::NOT_FOUND.into_response())
        }
    }
}
