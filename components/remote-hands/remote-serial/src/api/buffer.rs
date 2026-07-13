// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use super::ExtractSerial;
use aide::transform::TransformOperation;
use axum::body::Bytes;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use tracing::warn;

/// GET handler to return serial buffer content.
pub async fn handle_get_buffer(ExtractSerial(_, serial): ExtractSerial) -> Response {
    let ring = serial.ring.read().await;
    let data = ring.snapshot_all();
    drop(ring);

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/octet-stream")],
        Bytes::from(data),
    )
        .into_response()
}

/// Documentation for GET handler.
pub fn handle_get_buffer_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Read from serial buffer")
        .description("Obtain the serial buffer")
        .response_with::<{ StatusCode::OK.as_u16() }, Bytes, _>(|op| {
            op.description("Bytes red from serial buffer")
        })
}

/// DELETE handler to clear serial buffer content.
pub async fn handle_delete_buffer(ExtractSerial(_, serial): ExtractSerial) -> Response {
    {
        let mut ring = serial.ring.write().await;
        ring.clear();
    }

    StatusCode::NO_CONTENT.into_response()
}

/// Documentation for DELETE handler.
pub fn handle_delete_buffer_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Clear serial buffer")
        .description("Clear the serial buffer")
        .response::<{ StatusCode::NO_CONTENT.as_u16() }, ()>()
}

/// POST handler to write to serial buffer.
pub async fn handle_post_buffer(ExtractSerial(_, serial): ExtractSerial, body: Bytes) -> Response {
    match serial.write_tx.send(body.to_vec()).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            warn!(error = ?e, "Failed to write to serial buffer");
            (StatusCode::SERVICE_UNAVAILABLE, "writer down").into_response()
        }
    }
}

/// Documentation for POST handler.
pub fn handle_post_buffer_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Write to serial buffer")
        .description("Write to serial buffer")
        .response::<{ StatusCode::NO_CONTENT.as_u16() }, ()>()
        .response_with::<{ StatusCode::SERVICE_UNAVAILABLE.as_u16() }, (), _>(|op| {
            op.description("Internal error")
        })
}
