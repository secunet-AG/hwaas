// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use axum::http::Request;
use axum_tracing_opentelemetry::{
    middleware::{OtelAxumLayer, OtelInResponseLayer},
    tracing_opentelemetry_instrumentation_sdk::find_current_trace_id,
};
use tower::{layer::util::Stack, ServiceBuilder};
use tower_http::{
    classify::{ServerErrorsAsFailures, SharedClassifier},
    trace::{MakeSpan, TraceLayer},
};

type HuntLayers<L> = Stack<
    TraceLayer<SharedClassifier<ServerErrorsAsFailures>, TraceIdSpan>,
    Stack<OtelInResponseLayer, Stack<OtelAxumLayer, L>>,
>;
/// Extends the current span created by [`OtelAxumLayer`] by the `trace_id`.
pub struct TraceIdSpan;

impl<B> MakeSpan<B> for TraceIdSpan {
    fn make_span(&mut self, _request: &Request<B>) -> tracing::Span {
        tracing::span::Span::current()
            .record("trace_id", find_current_trace_id())
            .clone()
    }
}

/// Extension trait that adds methods to [`tower::ServiceBuilder`] for adding middleware from
/// hunt.
///
/// # Example
///
/// ```rust
/// use http::{Request, Response, header::HeaderName};
/// use bytes::Bytes;
/// use http_body_util::Full;
/// use std::{time::Duration, convert::Infallible};
/// use tower::{ServiceBuilder, ServiceExt, Service};
/// use tower_http::ServiceBuilderExt;
///
/// async fn handle(request: Request<Full<Bytes>>) -> Result<Response<Full<Bytes>>, Infallible> {
///     Ok(Response::new(Full::default()))
/// }
///
/// # #[tokio::main]
/// # async fn main() {
/// let service = ServiceBuilder::new()
///     // Methods from hunt
///     .hunt_tracing()
///     // Method from tower
///     .timeout(Duration::from_secs(30))
///     .service_fn(handle);
/// # let mut service = service;
/// # service.ready().await.unwrap().call(Request::new(Full::default())).await.unwrap();
/// # }
/// ```
pub trait ServiceBuilderExt<L> {
    /// Enable opentelemetry tracing.
    fn hunt_tracing(self) -> ServiceBuilder<HuntLayers<L>>;
}

impl<L> ServiceBuilderExt<L> for ServiceBuilder<L> {
    fn hunt_tracing(self) -> ServiceBuilder<HuntLayers<L>> {
        self.layer(OtelAxumLayer::default())
            .layer(OtelInResponseLayer)
            .layer(TraceLayer::new_for_http().make_span_with(TraceIdSpan))
    }
}
