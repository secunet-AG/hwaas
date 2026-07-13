// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

mod connection_handler;
mod error;
mod interface_handler_task;
mod interface_streams;
mod net_dev;
mod network_selector;
pub mod peer;
mod socket_handler_task;

use crate::connection_handler::ConnectionHandler;
use crate::interface_streams::InterfaceStreams;
pub use crate::network_selector::NetworkSelector;
use crate::peer::{Peer, PeerID};
use crate::socket_handler_task::SocketHandlerTask;
use aide::axum::routing::get_with;
use aide::axum::ApiRouter;
use aide::axum::IntoApiResponse;
use aide::openapi::{Info, OpenApi};
use aide::NoApi;
use axum::extract::ws::WebSocket;
use axum::extract::{ConnectInfo, Path, State, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::Router;
use axum_extra::headers::Header;
use axum_extra::typed_header::TypedHeader;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::net::SocketAddr;
use tokio::task::Builder;
use tower_http::request_id::SetRequestIdLayer;
use tower_http::trace::TraceLayer;
use tracing::{debug, instrument, Instrument};

// A type-safe path
#[derive(Deserialize, Serialize, Debug, JsonSchema)]
struct PathViaNetworkSelection {
    network: NetworkSelector,
}

pub fn get_router(interface_prefix: String) -> Router<()> {
    let conn_handler = ConnectionHandler::new(interface_prefix);

    let mut open_api = OpenApi {
        info: Info {
            description: Some("HWaaS WebSocket Gateway API".to_string()),
            version: env!("CARGO_PKG_VERSION").to_string(),
            ..Info::default()
        },
        ..Default::default()
    };

    ApiRouter::new()
        .api_route(
            "/ws/:network",
            get_with(handler, |op| {
                op.description(
                    "Establish a websocket connection to the network. \
                    The websocket transports L2 traffic (ethernet frames). \
                    Sending a message injects packets. \
                    Receiving messages equals to receiving L2 Network Packets",
                )
                .summary("Connect to network via websocket")
                .tag("Network API")
            }),
        )
        .layer(TraceLayer::new_for_http())
        // This middle set an ID per request.
        // Makes it easier to follow the debug output for more than one client connected.
        .layer(SetRequestIdLayer::new(
            PeerID::name().clone(),
            Peer::default(),
        ))
        .with_state(conn_handler)
        .finish_api(&mut open_api)
}

#[tracing::instrument(skip(ws, conn, peer, addr))]
#[axum::debug_handler]
async fn handler(
    State(conn): State<ConnectionHandler>,
    TypedHeader(peer): TypedHeader<PeerID>,
    NoApi(ConnectInfo(addr)): NoApi<ConnectInfo<SocketAddr>>,
    Path(PathViaNetworkSelection { network }): Path<PathViaNetworkSelection>,
    ws: WebSocketUpgrade,
) -> impl IntoApiResponse {
    debug!("Received new connection request from {}", addr);

    match conn.get_or_create(&network).await {
        Ok(is) => ws.on_upgrade(|s| handle_socket(s, is, peer)),
        Err(e) => e.into_response(),
    }
}

#[instrument(skip(socket))]
async fn handle_socket(socket: WebSocket, is: InterfaceStreams, peer: PeerID) {
    let sht = SocketHandlerTask::new(socket, is, peer);
    let _ = Builder::new()
        .name(sht.to_string().as_str())
        .spawn(sht.start())
        .in_current_span();
}
