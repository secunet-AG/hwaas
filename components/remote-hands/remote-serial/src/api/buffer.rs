// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use aide::transform::TransformOperation;
use axum::body::Bytes;

use super::ExtractSerial;

/// GET handler to return serial buffer content.
pub async fn handle_get_buffer(ExtractSerial(serial): ExtractSerial) -> Vec<u8> {
    serial.get_buffer()
}

/// Documentation for GET handler.
pub fn handle_get_buffer_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Read from serial buffer")
        .description("Obtain the serial buffer")
}

/// DELETE handler to clear serial buffer content.
pub async fn handle_delete_buffer(ExtractSerial(serial): ExtractSerial) {
    serial.clear_buffer()
}

/// Documentation for DELETE handler.
pub fn handle_delete_buffer_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Clear serial buffer")
        .description("Clear the serial buffer")
}

/// POST handler to write to serial buffer.
pub async fn handle_post_buffer(
    ExtractSerial(serial): ExtractSerial,
    body: Bytes,
) -> Result<(), ()> {
    serial.write(body.to_vec()).await
}

/// Documentation for POST handler.
pub fn handle_post_buffer_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Write to serial buffer")
        .description("Write to serial buffer")
}
