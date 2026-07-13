// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use axum::body::Bytes;
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use tracing::{error, trace, warn};

/// Send an HTTP request with the given parameters to an auxiliary device.
pub async fn send_aux_request(
    method: Method,
    url: Uri,
    headers: HeaderMap,
    body: Option<Bytes>,
) -> Result<Response, (StatusCode, &'static str)> {
    // Build request with url and method
    let mut req = hyper::Request::builder().uri(url).method(method);

    // Insert headers to request
    insert_response_headers(req.headers_mut(), headers)?;

    // Add body to request
    let req = req.body(body.unwrap_or_default()).map_err(|e| {
        warn!("Auxiliary Device request failed: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Error occurred while forwarding the request internally",
        )
    })?;

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
        .execute(req.try_into().map_err(|e| {
            warn!("Cannot send HTTP request: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Cannot send HTTP request",
            )
        })?)
        .await
        .map_err(|e| {
            warn!("Auxiliary Device request failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Error occurred while forwarding the request internally",
            )
        })?;
    trace!("send_aux_request res: {:?}", response.status());

    Ok(http::Response::from(response).into_response())
}

/// Helper function to insert headers into a HeaderMap
fn insert_response_headers(
    hdrs: Option<&mut HeaderMap>,
    headers: HeaderMap,
) -> Result<(), (StatusCode, &'static str)> {
    let Some(hdrs) = hdrs else {
        error!("Failed to build response: could not get headers (ResponseBuilder error)");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Could not build response",
        ));
    };

    for (k, v) in headers {
        if let Some(k) = k {
            hdrs.insert(k, v);
        } else {
            warn!("Empty header key for value: {:?}", v);
        }
    }

    Ok(())
}
