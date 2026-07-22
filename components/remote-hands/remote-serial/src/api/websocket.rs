// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use aide::transform::TransformOperation;
use axum::{
    extract::ws::{Message, WebSocketUpgrade},
    response::Response,
};
use tokio::select;
use tracing::{error, instrument};

use super::ExtractSerial;

#[instrument(skip_all)]
/// GET handler for websocket API.
pub async fn handle_websocket(
    ExtractSerial(serial): ExtractSerial,
    websocket_upgrade: WebSocketUpgrade,
) -> Response {
    websocket_upgrade.on_upgrade(|mut websocket| async move {
        let mut receiver = serial.subscribe();
        loop {
            #[rustfmt::skip]
            select! {
		// Forward from serial to websocket
		Some(data) = receiver.recv() => {
                    if websocket.send(Message::Binary(data)).await.is_err() {
			return;
                    }
		}
		// Forward from websocket to serial
		Some(msg) = websocket.recv() => {
                    match msg {
			Ok(Message::Ping(data)) =>
			    if websocket.send(Message::Pong(data)).await.is_err() {
				return;
			    },
			Ok(Message::Pong(_)) => {}
			Ok(Message::Binary(data)) =>
			    if serial.write(data).await.is_err() {
				return;
			    },
			Ok(Message::Text(data)) => {
			    if serial.write(data.into_bytes()).await.is_err() {
				return;
			    }
			}
			Ok(Message::Close(_)) =>
			    return,
			Err(e) => {
			    error!(error = ?e, "websocket recv");
			    return;
			}
                    }
		}
            };
        }
    })
}

/// Documentation for GET handler.
pub fn handle_websocket_doc(op: TransformOperation) -> TransformOperation {
    op.description("Establish a WebSocket connection with the serial peripheral")
        .summary("Connect to serial via WebSocket")
}
