// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

// TODO: FIXME: The test contianed in the README are not able to run as they are.
//#![doc = include_str!("../README.md")]
use std::net::SocketAddr;

use axum::Router;
use axum::extract::connect_info::IntoMakeServiceWithConnectInfo;
use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};
use tower_http::trace::TraceLayer;

pub use header_injector::inject_headers;
pub use hunt::{Hunt, HuntBuilder};
#[cfg(feature = "reqwest")]
pub use reqwest_inject_middleware::ReqwestInjectMiddleware;

mod header_injector;
mod hunt;
#[cfg(feature = "reqwest")]
mod reqwest_inject_middleware;
#[cfg(feature = "tower")]
mod tower_layer;

/// Opinionated helper for HWaaS axum based services.
/// Takes a [Router] (more precisely a `Router<()>`) and wrap it by OpenTelemetry related layers.
/// Finally return a ready to serve tower service.
pub fn hunt_axum_router(router: Router) -> IntoMakeServiceWithConnectInfo<Router, SocketAddr> {
    router
        .layer(TraceLayer::new_for_http())
        .layer(OtelInResponseLayer)
        .layer(OtelAxumLayer::default())
        .into_make_service_with_connect_info::<SocketAddr>()
}
