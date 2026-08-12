// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use axum::body::Bytes;
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::Response;
use reqwest::Url;
use tracing::{trace, warn};

/// Send an HTTP request with the given parameters to an auxiliary device.
pub async fn send_aux_request(
    method: Method,
    url: Url,
    headers: HeaderMap,
    body: Option<Bytes>,
) -> Result<Response, (StatusCode, &'static str)> {
    // Execute request and await response
    let response = reqwest::Client::builder()
        .build()
        .map_err(|e| {
            warn!("Cannot build HTTP client: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Cannot build HTTP client",
            )
        })?
        .request(method, url)
        .headers(headers)
        .body(body.unwrap_or_default())
        .send()
        .await
        .map_err(|e| {
            warn!("Auxiliary Device request failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Error occurred while forwarding the request internally",
            )
        })?;
    trace!("send_aux_request res: {:?}", response.status());

    // Inspired by: <https://github.com/tokio-rs/axum/blob/151cd5c12325373b86daf405a6afc0a0086a6706/examples/reqwest-response/src/main.rs>
    let mut response_builder = axum::response::Response::builder().status(response.status());
    if let Some(hdr_map) = response_builder.headers_mut() {
        *hdr_map = response.headers().clone();
    }
    response_builder
        .body(axum::body::Body::from(response.bytes().await.unwrap()))
        .map_err(|e| {
            warn!(
                "failed to translate auxiliary device response into valid format: {:?}",
                e
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Error while processing auxiliary device response",
            )
        })
}
