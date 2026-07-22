// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

mod buffer;
mod list;
mod websocket;

use aide::{
    axum::{
        routing::{get_with, post_with},
        ApiRouter,
    },
    openapi::OpenApi,
    transform::TransformPathItem,
    OperationIo,
};
use axum::{
    async_trait,
    extract::{FromRequestParts, Path},
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    RequestPartsExt, Router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, convert::Infallible};

use crate::{app_state::AppState, serial::SerialState};
use buffer::{
    handle_delete_buffer, handle_delete_buffer_doc, handle_get_buffer, handle_get_buffer_doc,
    handle_post_buffer, handle_post_buffer_doc,
};
use list::{handle_get_all, handle_get_all_doc, handle_reset, handle_reset_doc};
use websocket::{handle_websocket, handle_websocket_doc};

#[derive(Clone, Deserialize, Serialize, JsonSchema)]
/// The ID in the API to specify the name of the serial interface to use.
pub struct SerialID {
    /// Name of the serial interface.
    serial_interface: String,
}

#[derive(OperationIo)]
#[aide(input_with = "Path<SerialID>")]
/// Struct with the `SerialState` retrieved by the given `SerialID`.
pub struct ExtractSerial(pub SerialState);

/// Trait to allow all services that want to expose the serial API to have a
/// default interface to query for parts of the state.
/// Currently this is `remote-serial`, as well as `remote-usb`.
#[async_trait]
pub trait HasSerial {
    /// Return the SerialState for the given serial_id.
    async fn get_serial(&self, id: &'_ str) -> Option<SerialState>;
    /// Return all known SerialStates.
    async fn get_serials(&self) -> Vec<SerialState>;
    /// Return a list of serial ids from the state.
    async fn get_serial_ids(&self) -> Vec<String>;
}

/// Implementation of the trait functions for `remote-serial`s `AppState`.
#[async_trait]
impl HasSerial for AppState {
    /// Return the SerialState for the given serial_id.
    async fn get_serial(&self, id: &'_ str) -> Option<SerialState> {
        self.serials.get(id).cloned()
    }
    /// Return all known SerialStates.
    async fn get_serials(&self) -> Vec<SerialState> {
        HashMap::clone(&self.serials).values().cloned().collect()
    }
    /// Return all known serial ids.
    async fn get_serial_ids(&self) -> Vec<String> {
        self.serials.keys().cloned().collect()
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for ExtractSerial
where
    S: Send + Sync + HasSerial,
{
    type Rejection = Response;

    /// Split the parts of the request and create structs as needed in the API handling.
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path(SerialID {
            serial_interface: serial_id,
        }) = parts.extract().await.map_err(IntoResponse::into_response)?;
        if let Some(serial) = state.get_serial(&serial_id).await {
            Ok(ExtractSerial(serial))
        } else {
            Err(StatusCode::NOT_FOUND.into_response())
        }
    }
}

/// Serial router with all sub routes.
pub fn serial_router<S>() -> ApiRouter<S>
where
    S: HasSerial + Clone + Send + Sync + 'static,
{
    ApiRouter::new()
        .api_route_with(
            "/serial",
            get_with(handle_get_all::<S>, handle_get_all_doc),
            serial_api_doc,
        )
        .api_route_with(
            "/serial/reset",
            post_with(handle_reset::<S>, handle_reset_doc),
            serial_api_doc,
        )
        .api_route_with(
            "/serial/:serial_interface",
            get_with(handle_get_buffer, handle_get_buffer_doc)
                .delete_with(handle_delete_buffer, handle_delete_buffer_doc)
                // PUT is handled for compatibility reasons
                // while POST actually has the right
                // semantics.
                .put_with(handle_post_buffer, handle_post_buffer_doc)
                .post_with(handle_post_buffer, handle_post_buffer_doc),
            serial_api_doc,
        )
        .api_route_with(
            "/serial/:serial_interface/websocket",
            get_with(handle_websocket, handle_websocket_doc),
            serial_api_doc,
        )
}

/// Group all serial API endpoints with the same tag.
fn serial_api_doc(op: TransformPathItem) -> TransformPathItem {
    op.tag("Serial API")
}

/// Create the router for the `SerialAPI`
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
        "remote-hands serial service",
        env!("CARGO_PKG_VERSION"),
        |router| async { Ok::<_, Infallible>(router.merge(serial_router())) },
    )
    .await?;
    let router = router.with_state(state);
    Ok((router, api))
}
