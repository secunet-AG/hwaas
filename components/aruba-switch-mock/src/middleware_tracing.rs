// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use axum::body::Body;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use tracing::debug;

pub(crate) async fn route_tracing(req: Request<Body>, next: Next) -> Response {
    // get URL id
    let url = req.uri().to_string();
    let methode = req.method();

    debug!("Start processing request {} {}", methode, url);
    next.run(req).await
}
