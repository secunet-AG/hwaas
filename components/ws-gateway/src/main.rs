// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

//! Websocket Gateway
//!
//! This application is supposed to transfer L2-Network Traffic via a WebSocket connection.
//! This WS connection is established between the server and the client part (corresponding to the websocket role).
//! Both parts forward traffic to the Linux kernel networking Stack via AF_PACKET.

use clap::Parser;
use hunt::{HuntBuilder, hunt_axum_router};
use std::net::SocketAddr;
use std::process::exit;
use tokio::net::TcpListener;
use tracing::{error, info};
use ws_gateway_lib::get_router;

/// bridge l2 networks via websocket
#[derive(Parser, Debug, Clone)]
#[clap(author, version, about, long_about = None)]
#[clap(propagate_version = true)]
pub(crate) struct CliArgs {
    /// level of verbosity
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// The socket address to listen for new websocket connections
    #[arg(default_value = "127.0.0.1:9002")]
    address: SocketAddr,

    /// Prefix for VLAN interfaces.
    /// This prefix will be appended with an ID (e.g. 'vlan' -> vlan1, vlan2, ...).
    #[arg(long, short, default_value = "wsn")]
    dev: String,

    /// Specifying this Socket Address enables tokio console.
    /// See `<https://github.com/tokio-rs/console>`
    tokio_console_address: Option<SocketAddr>,
}

#[tokio::main]
async fn main() -> Result<(), ()> {
    let args: CliArgs = CliArgs::parse();

    let _hunt = HuntBuilder::new()
        .verbosity(args.verbose)
        .tokio_console_address(args.tokio_console_address)
        .enable_otel_layer()
        .append_filters(vec!["ws_gateway"])
        .fallback_name(env!("CARGO_PKG_NAME"))
        .fallback_version(env!("CARGO_PKG_VERSION"))
        .build();

    info!("gateway listening at {}", args.address);
    start(&args.address, args.dev).await
}

async fn start(addr: &SocketAddr, interface_prefix: String) -> Result<(), ()> {
    let service = hunt_axum_router(get_router(interface_prefix));

    // channel to trigger server shutdown
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    tokio::spawn(async {
        // override the default SIGINT signal handler for the runtime of this app
        // SIGTERM does not have any special handler. therefore, it has the default behavior.
        tokio::signal::ctrl_c().await.unwrap();
        let _ = tx
            .send(())
            .map_err(|_| error!("failed to initiate graceful shutdown"));

        // exit immediately when 2nd SIGINT is received
        tokio::signal::ctrl_c().await.unwrap();
        exit(1);
    });

    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| error!(error = ?e, "could not bind to {}", addr))?;

    axum::serve(listener, service)
        .with_graceful_shutdown(async {
            // wait for SIGINT signal to initiate shutdown
            rx.await.ok();
            info!("Shutting down...");
        })
        .await
        .map_err(|e| {
            error!(error.dbg = ?e, error.msg = %e, "server error");
        })
}
