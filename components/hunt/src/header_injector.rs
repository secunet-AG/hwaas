// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use axum::http::{HeaderMap, HeaderName};
use opentelemetry::propagation::TextMapPropagator;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use tracing::warn;
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// The HeaderInjector knows how to inject headers into a (e.g. reqwest) HeaderMap in an OpenTelemetry
/// determined way. The concrete headers originate from a propagator.
struct HeaderInjector<'a>(&'a mut HeaderMap);
impl opentelemetry::propagation::Injector for HeaderInjector<'_> {
    /// Add an HTTP header key and value to the requests sent to extern service.
    fn set(&mut self, key: &str, value: String) {
        if let (Ok(k), Ok(v)) = (key.try_into(), value.try_into()) {
            self.0.insert::<HeaderName>(k, v);
        } else {
            warn!("Could not inject trace HTTP headers")
        }
    }
}

/// This function uses the [`HeaderInjector`] in conjunction with the [`TraceContextPropagator`]
/// to inject the currently valid `traceparent` and `tracestate` HTTP headers.
pub fn inject_headers(hdrs: &mut HeaderMap) {
    let mut injector = HeaderInjector(hdrs);
    let context = tracing::Span::current().context();
    TraceContextPropagator::new().inject_context(&context, &mut injector);
}
