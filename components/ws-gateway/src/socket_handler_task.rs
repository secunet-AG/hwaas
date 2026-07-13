// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::interface_streams::InterfaceStreams;
use crate::peer::PeerID;
use axum::extract::ws::{Message, WebSocket};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use std::fmt::{Debug, Display, Formatter};
use tokio::select;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::broadcast::Receiver;
use tokio::sync::mpsc::Sender;
use tracing::{debug, instrument, warn};

pub(crate) struct SocketHandlerTask {
    ws: WebSocket,
    is: InterfaceStreams,
    peer: PeerID,
}

impl Display for SocketHandlerTask {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "SHT[{};{}]",
            self.is.get_iht_name(),
            self.peer
        ))
    }
}

impl Debug for SocketHandlerTask {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl SocketHandlerTask {
    pub fn new(ws: WebSocket, is: InterfaceStreams, peer: PeerID) -> Self {
        Self { ws, is, peer }
    }

    #[instrument]
    pub async fn start(self) {
        // split websocket into read and write parts to handel them concurrently
        let (ws_tx, ws_rx) = self.ws.split();
        let (rx, tx) = self.is.get();

        // spawn concurrent handler tasks
        // if one task finishes the other one is aborted
        select! {
            _ = Self::handle_tx(ws_tx, rx) => debug!("tx terminated"),
            _ = Self::handle_rx(ws_rx, tx) => debug!("rx terminated")
        };
    }

    async fn handle_tx(mut ws_tx: SplitSink<WebSocket, Message>, mut rx: Receiver<Message>) {
        loop {
            match rx.recv().await {
                Ok(m) => {
                    if let Err(e) = ws_tx.send(m).await {
                        warn!("Could not send message: {:?}", e)
                    }
                }
                Err(RecvError::Closed) => {
                    return;
                }
                Err(RecvError::Lagged(num)) => {
                    debug!("lagged and hence missed {} packages", num)
                }
            }
        }
    }

    async fn handle_rx(mut ws_rx: SplitStream<WebSocket>, tx: Sender<Message>) {
        while let Some(m) = ws_rx.next().await {
            match m {
                Ok(msg) => {
                    if let Err(e) = tx.send(msg).await {
                        warn!("Could not write to mpsc channel: {:?}", e)
                    };
                }
                Err(e) => {
                    warn!("could not get message from ws: {:?}", e);
                    break;
                }
            };
        }
    }
}
