// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use super::ExtractSerial;
use crate::serial::serial_state::SerialState;
use aide::transform::TransformOperation;
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
};
use futures::{SinkExt, StreamExt};
use tokio::sync::broadcast;
use tracing::instrument;

/// WebSocket handler for `/serial/{id}/websocket`.
///
/// Behavior:
/// - On connect, the stream starts at the current end of the ring buffer
///   (i.e. only new data is sent).
/// - Whenever new data arrives, it is streamed to the client as binary frames.
/// - If the client is too slow and falls behind the ring buffer, a warning is
///   logged and the stream skips forward.
/// - Incoming WebSocket messages (binary or text) are forwarded to the serial
///   writer task.
#[instrument(skip_all, fields(serial_id=?_serial_id))]
pub async fn handle_websocket(
    ExtractSerial(_serial_id, serial): ExtractSerial,
    websocket_upgrade: WebSocketUpgrade,
) -> Response {
    websocket_upgrade.on_upgrade(move |socket| ws_loop(serial, socket))
}

/// WebSocket loop:
/// - outbound: forward broadcast chunks as binary frames
/// - inbound: forward client messages to the serial writer channel
///
/// Lag handling:
/// - If the client is too slow, `broadcast` reports `RecvError::Lagged(n)`.
/// - We log a warning and continue; the ring buffer continues regardless.
pub async fn ws_loop(st: SerialState, socket: WebSocket) {
    let (mut ws_tx, mut ws_rx) = socket.split();

    // Subscribe to live stream from "now"
    let mut live_rx = st.ws_tx.subscribe();

    // Inbound (client -> serial writer)
    let write_tx = st.write_tx.clone();
    let inbound = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_rx.next().await {
            match msg {
                Message::Binary(b) => {
                    let _ = write_tx.send(b).await;
                }
                Message::Text(s) => {
                    let _ = write_tx.send(s.into_bytes()).await;
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
        tracing::debug!("ws inbound ended");
    });

    // Outbound (serial -> client)
    let mut warned = false;
    loop {
        match live_rx.recv().await {
            Ok(chunk) => {
                if ws_tx.send(Message::Binary(chunk.to_vec())).await.is_err() {
                    break;
                }
            }
            Err(broadcast::error::RecvError::Lagged(n)) => {
                // Client fell behind broadcast capacity; ring continues.
                if !warned {
                    tracing::warn!(dropped=%n, "websocket client too slow (broadcast lagged; no further warning for this client)");
                    warned = true;
                }
                continue;
            }
            Err(broadcast::error::RecvError::Closed) => {
                tracing::warn!("broadcast channel closed");
                break;
            }
        }
    }

    inbound.abort();
}

/// Documentation for WebSocket handler.
pub fn handle_websocket_doc(op: TransformOperation) -> TransformOperation {
    op.description("Establish a WebSocket connection with the serial peripheral")
        .summary("Connect to serial via WebSocket")
}
