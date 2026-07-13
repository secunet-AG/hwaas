// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

pub mod activation_info;
mod auxiliaries;
mod auxiliary;
mod auxiliary_api;
mod reverse_proxy;

use crate::app_state::{AppState, DeviceState};
use aide::{openapi::OpenApi, OperationIo};
use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts, Path, Query},
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    RequestPartsExt, Router,
};
use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashMap;
use std::convert::Infallible;

use auxiliaries::all_aux_router;
use auxiliary::{aux_device_activation_router, aux_device_router};
use auxiliary_api::aux_device_api_router;

/// Create the router for the `AuxiliaryAPI`
pub async fn get_router<S>(state: AppState) -> Result<Router<S>, Infallible> {
    Ok(prepare_api_router(state).await?.0)
}

/// Build a `OpenAPI` based on the router implementation
pub async fn get_api<S>(state: AppState) -> Result<OpenApi, Infallible> {
    Ok(prepare_api_router::<S>(state).await?.1)
}

/// Prepare router with all sub routes.
pub async fn prepare_api_router<S>(state: AppState) -> Result<(Router<S>, OpenApi), Infallible> {
    let all_aux_router = all_aux_router();
    let aux_device_router = aux_device_router();
    let aux_device_activation_router = aux_device_activation_router();
    let aux_device_api_router = aux_device_api_router();
    let (router, api) = remote_axum::api_router(
        "remote-hands auxiliary device service",
        env!("CARGO_PKG_VERSION"),
        |router| async {
            let router = router
                .merge(all_aux_router)
                .merge(aux_device_router)
                .merge(aux_device_activation_router)
                .merge(aux_device_api_router);
            Ok::<_, Infallible>(router)
        },
    )
    .await?;
    let router = router.with_state(state);
    Ok((router, api))
}

/// The ID in the API to specify the name of the auxiliary device to use.
#[derive(Clone, Deserialize, JsonSchema)]
pub struct AuxiliaryID {
    /// Name of the auxiliary device.
    pub device_id: String,
}

#[derive(OperationIo)]
#[aide(input_with = "Path<AuxiliaryID>")]
/// Struct with the `DeviceState` retrieved by the given `AuxiliaryID`, as well
/// as potential query parameters that are part of the request.
pub struct ExtractAux {
    pub state: DeviceState,
    pub query: HashMap<String, String>,
}

#[async_trait]
impl<S> FromRequestParts<S> for ExtractAux
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    /// Split the parts of the request and create structs as needed in the API handling.
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Extract auxiliary id
        let Path(AuxiliaryID { device_id: aux_id }) =
            parts.extract().await.map_err(IntoResponse::into_response)?;

        // Extract query parameters if given
        let query_params = parts
            .extract::<Query<HashMap<String, String>>>()
            .await
            .map(|Query(params)| params)
            .map_err(|err| err.into_response())?;

        // Build `ExtractAux` by querying the `AppState` with the aux_id
        if let Some(aux) = AppState::from_ref(state).aux_devices.get(&aux_id).cloned() {
            Ok(ExtractAux {
                state: aux,
                query: query_params.clone(),
            })
        } else {
            Err(StatusCode::NOT_FOUND.into_response())
        }
    }
}
