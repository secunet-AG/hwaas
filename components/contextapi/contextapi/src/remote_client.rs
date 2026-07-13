// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use axum::body::Body;
use axum::http::Response;
use axum::http::StatusCode;

/// Re-export the remote client from the remote_client crate.
pub(crate) use remote_client::RemoteClient;

pub fn reqwest_to_axum_response(
    mut response: reqwest::Response,
) -> Result<axum::response::Response, (StatusCode, &'static str)> {
    let mut response_builder = Response::builder().status(response.status());
    if let Some(headers) = response_builder.headers_mut() {
        std::mem::swap(headers, response.headers_mut());
    }
    response_builder
        .body(Body::from_stream(response.bytes_stream()))
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Unable to proxy response",
            )
        })
}
