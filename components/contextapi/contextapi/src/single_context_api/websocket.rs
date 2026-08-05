// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use axum::extract::ws::CloseFrame;
use axum::extract::ws::Message;
use axum::extract::ws::WebSocket;
use axum::http::Uri;
use futures::SinkExt;
use futures::StreamExt;
use futures::try_join;
use std::future::Future;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Error as TungsteniteError;
use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::protocol::CloseFrame as TungsteniteCloseFrame;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};
use tracing::error;

enum WsError {
    ClientServerError(axum::Error),
    GatewayError(Box<TungsteniteError>),
    CloseWs,
    WsFramesUnsupported,
}

/// Connects two websockets with each other bidirectionally.
/// The connection may be terminated externally through the deletion signal.
pub(crate) async fn connect_websockets(
    ws_stream: WebSocket,
    ws_gateway: WebSocketStream<MaybeTlsStream<TcpStream>>,
    websocket_deletion_signal: impl Future<Output = ()> + Send + 'static,
) {
    let axum_msg_to_tungstenite = |msg| match msg {
        Message::Text(txt) => TungsteniteMessage::Text(txt),
        Message::Binary(binary) => TungsteniteMessage::Binary(binary),
        Message::Ping(ping) => TungsteniteMessage::Ping(ping),
        Message::Pong(pong) => TungsteniteMessage::Pong(pong),
        Message::Close(close_frame) => {
            TungsteniteMessage::Close(close_frame.map(|close_frame| TungsteniteCloseFrame {
                reason: close_frame.reason,
                code: close_frame.code.into(),
            }))
        }
    };

    let tungstenite_to_axum_msg = |msg: TungsteniteMessage| match msg {
        TungsteniteMessage::Text(txt) => Some(Message::Text(txt)),
        TungsteniteMessage::Binary(binary) => Some(Message::Binary(binary)),
        TungsteniteMessage::Ping(ping) => Some(Message::Ping(ping)),
        TungsteniteMessage::Pong(pong) => Some(Message::Pong(pong)),
        TungsteniteMessage::Close(close_frame) => {
            Some(Message::Close(close_frame.map(|close_frame| CloseFrame {
                reason: close_frame.reason,
                code: close_frame.code.into(),
            })))
        }
        TungsteniteMessage::Frame(frame) => {
            // Axum does not support websocket frames
            error!(?frame, "received unsupported websocket frame");
            None
        }
    };

    let (sink1, stream1) = ws_stream.split();
    let (sink2, stream2) = ws_gateway.split();

    let stream1 = stream1.map(|msg| {
        msg.map(axum_msg_to_tungstenite)
            .map_err(WsError::ClientServerError)
    });
    let mut sink2 = sink2.sink_map_err(|e: tokio_tungstenite::tungstenite::Error| {
        WsError::GatewayError(Box::new(e))
    });
    let client_to_gateway = stream1.forward(&mut sink2);

    let stream2 = stream2.map(|msg| match msg.map(tungstenite_to_axum_msg) {
        Ok(Some(axum_msg)) => Ok(axum_msg),
        Ok(None) => Err(WsError::WsFramesUnsupported),
        Err(e) => Err(WsError::GatewayError(Box::new(e))),
    });
    let mut sink1 = sink1.sink_map_err(WsError::ClientServerError);
    let gateway_to_client = stream2.forward(&mut sink1);

    let res = try_join!(client_to_gateway, gateway_to_client, async move {
        websocket_deletion_signal.await;
        Result::<(), _>::Err(WsError::CloseWs)
    });

    match res {
        Ok(_) | Err(WsError::CloseWs) => (),
        Err(WsError::GatewayError(e)) => match *e {
            TungsteniteError::ConnectionClosed => (),
            _ => error!(error.dbg = ?e, "error occurred when working with the websocket gateway"),
        },
        Err(WsError::ClientServerError(e)) => {
            error!(error.dbg = ?e, "error occurred when handling the websocket")
        }
        Err(WsError::WsFramesUnsupported) => {
            error!("websocket frame feature not supported");
        }
    }

    let res = sink1.close().await;
    if let Err(WsError::ClientServerError(e)) = res {
        error!(error.dbg = ?e, error.msg = %e, "error occurred while closing the websocket sink1")
    }
    let res = sink2.close().await;
    if let Err(WsError::ClientServerError(e)) = res {
        error!(error.dbg = ?e, error.msg = %e, "error occurred while closing the websocket sink2")
    }
}

/// Create a websocket connection to the given URI.
pub async fn create_websocket(uri: Uri) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, ()> {
    let sock = uri
        .clone()
        .into_client_request()
        .map(|mut r| {
            hunt::inject_headers(r.headers_mut());
            r
        })
        .map_err(|e| error!(error.dbg = ?e, error.msg = %e, "could not build client request"))?;

    connect_async(sock)
        .await
        .map_err(|e| error!(%uri, error.dbg = ?e, error.msg = %e, "could not connect to websocket"))
        .map(|(stream, _)| stream)
}
