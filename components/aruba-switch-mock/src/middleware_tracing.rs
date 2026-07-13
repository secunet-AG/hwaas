// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use tracing::debug;

pub(crate) async fn route_tracing<B>(req: Request<B>, next: Next<B>) -> Response
where
    B: Send,
{
    // get URL id
    let url = req.uri().to_string();
    let methode = req.method();

    debug!("Start processing request {} {}", methode, url);
    next.run(req).await
}
