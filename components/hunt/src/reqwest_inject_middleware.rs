// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::inject_headers;
use axum::async_trait;
use axum::http::Extensions;
use reqwest::{Request, Response};
use reqwest_middleware::{Middleware, Next, Result};

pub struct ReqwestInjectMiddleware;

/// This is only useful if Hunt was instantiated with OpenTelemetry enabled.
/// If making a request from the current service towards another one requires
/// correct request headers.
/// This [`Middleware`] do so for [`reqwest_middleware`] clients.
///
/// If the client is set up with this middleware traceIDs and transparent headers are passed
/// automatically. The other service has to respect this information in order to be OpenTelemetry
/// and Hunt "compliant".
///
/// If the other service utilizes rust and the [`axum`] there is a helper for extracting the headers:
/// See [`crate::hunt_axum_router`].
#[async_trait]
impl Middleware for ReqwestInjectMiddleware {
    async fn handle(
        &self,
        mut req: Request,
        extensions: &mut Extensions,
        next: Next<'_>,
    ) -> Result<Response> {
        inject_headers(req.headers_mut());
        next.run(req, extensions).await
    }
}
