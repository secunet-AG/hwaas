// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use aide::axum::routing::{get_with, post_with, put_with};
use aide::axum::ApiRouter;
use aide::openapi::{Info, OpenApi};
use axum::http::{Method, StatusCode, Uri};
use axum::Router;
use tracing::debug;

use connection_handler::{ConnectionHandler, ConnectionHandlerError};
use inventory::{InventoryBackend, InventoryConnector};

use crate::app_state::AppState;
use crate::handlers::{get_switch_info, get_switches, handle_ports, setup_switch};

/// Initialize the `OpenAPI` with crate specific defaults
fn get_default_openapi() -> OpenApi {
    OpenApi {
        info: Info {
            description: Some("HWaaS NetCtrl API".to_string()),
            version: env!("CARGO_PKG_VERSION").to_string(),
            ..Info::default()
        },
        ..Default::default()
    }
}

/// Create the router for the `ContextAPI`
pub async fn get_router<S>(
    inventory: InventoryBackend,
) -> Result<Router<S>, ConnectionHandlerError> {
    let mut api = get_default_openapi();
    prepare_router(inventory, &mut api).await
}

/// Build a `OpenAPI` based on the router implementation
pub async fn get_api(inventory: InventoryBackend) -> OpenApi {
    let mut api = get_default_openapi();
    let _ = prepare_router::<()>(inventory, &mut api).await;
    api
}

/// Takes a [`InventoryBackend`] and a [`OpenApi`] to build and return a Router.
/// The [`OpenApi`] mutable reference is used to finalize the OAS.
/// The state type is generic over parameter `S`.
///
/// ## Parmas
/// * `inventory`: Some sort of [`InventoryBackend`]
/// * `api`: a mutable reference to a [`OpenApi`]. This is altered during finalisation of the [`aide::axum::ApiRouter`].
async fn prepare_router<S>(
    inventory: InventoryBackend,
    api: &mut OpenApi,
) -> Result<Router<S>, ConnectionHandlerError> {
    let app_state = AppState {
        connection_handler: Arc::new(
            ConnectionHandler::new(InventoryConnector::new(inventory)).await?,
        ),
    };

    Ok(ApiRouter::new()
        .api_route(
            "/switches",
            get_with(
                get_switches::get_switches,
                get_switches::api_doc_get_switches,
            ),
        )
        .api_route(
            "/switches/:switch_id",
            get_with(
                get_switch_info::get_switch_info,
                get_switch_info::api_doc_get_switch_info,
            ),
        )
        .api_route(
            "/switches/:switch_id/ports/:port_id",
            put_with(handle_ports::enable_port, handle_ports::api_doc_enable_port).delete_with(
                handle_ports::disable_port,
                handle_ports::api_doc_disable_port,
            ),
        )
        .api_route(
            "/switches/:switch_id/setup",
            post_with(
                setup_switch::setup_switch,
                setup_switch::api_doc_setup_switch,
            ),
        )
        .fallback(fallback_handler)
        .finish_api(api)
        .with_state(app_state))
}

async fn fallback_handler(method: Method, uri: Uri) -> (StatusCode, String) {
    debug!("Unhandled route: {}", uri);
    (
        StatusCode::NOT_FOUND,
        format!("No route for `{} {}`", method, uri),
    )
}
