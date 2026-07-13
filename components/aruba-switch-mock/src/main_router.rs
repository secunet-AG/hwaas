// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::app_state::AppState;
use crate::middleware_auth::check_auth;
use crate::middleware_route_call_stats::route_call_stats;
use axum::handler::Handler;
use axum::http::{Method, StatusCode, Uri};
use axum::routing::{get, post, put, IntoMakeService};
use axum::{middleware, Router};
use std::sync::Arc;
use tower_cookies::CookieManagerLayer;
use tracing::debug;

use crate::handlers::login::login;
use crate::handlers::logout::logout;
use crate::handlers::ports::{enable_port, ports};
use crate::handlers::root::handler;
use crate::handlers::root_auth::handler_auth;
use crate::handlers::stats::stats;
use crate::handlers::vlan_ports::vlan_ports;
use crate::middleware_tracing::route_tracing;

pub(crate) fn get_router() -> IntoMakeService<Router> {
    let app_state = Arc::new(AppState::default());

    let auth_layer = || middleware::from_fn_with_state(app_state.clone(), check_auth);

    let auth_router = Router::new()
        .route("/", get(handler_auth))
        .route("/ports", get(ports))
        .route("/ports/:port_id", put(enable_port))
        .route("/vlans-ports", get(vlan_ports))
        .with_state(app_state.clone())
        .layer(auth_layer())
        .fallback(fallback);

    Router::new()
        .route("/", get(handler))
        .route("/stats", get(stats))
        .route(
            "/rest/v1/login-sessions",
            post(login).delete(logout.layer(auth_layer())),
        )
        .nest("/rest/v1/", auth_router)
        .with_state(app_state.clone())
        .layer(middleware::from_fn_with_state(app_state, route_call_stats))
        .layer(CookieManagerLayer::new())
        .layer(middleware::from_fn(route_tracing))
        .fallback(fallback)
        .into_make_service()
}

async fn fallback(method: Method, uri: Uri) -> (StatusCode, String) {
    debug!("Unhandled route: {}", uri);
    (
        StatusCode::NOT_FOUND,
        format!("No route for `{} {}`", method, uri),
    )
}
