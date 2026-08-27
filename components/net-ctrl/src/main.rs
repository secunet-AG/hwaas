// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::exit;

use clap::Parser;
use hunt::{HuntBuilder, hunt_axum_router};
use sd_notify::NotifyState;
use tokio::net::TcpListener;
use tracing::{error, info, warn};

use net_ctrl_lib::get_router;
use net_ctrl_lib::{InventoryBackend, PullInventoryFileBackend};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
    /// path to the inventory file (json)
    #[arg(short, long, alias("config-file"))]
    inventory_file: PathBuf,

    /// if specified logging messages are written to this file (additionally to stdout)
    #[arg(short, long)]
    log_file: Option<PathBuf>,

    /// the socket address to listen on
    #[arg(default_value = "127.0.0.1:8080")]
    address: SocketAddr,

    /// level of verbosity (could be used several times; e.g. '-vv')
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

#[tokio::main]
async fn main() -> Result<(), ()> {
    let args: CliArgs = CliArgs::parse();

    let _hunt = HuntBuilder::new()
        .verbosity(args.verbose)
        .enable_otel_layer()
        .append_filters(vec![
            "network_type_ids",
            "connection_handler",
            "net_ctrl_lib",
            "switch",
            "inventory",
            "net_ctrl",
        ])
        //.set_logfile(args.log_file)
        .fallback_name(env!("CARGO_PKG_NAME"))
        .fallback_version(env!("CARGO_PKG_VERSION"))
        .build();

    // Store a mapping of SwitchID's to SwitchSelector's in an InventoryBackend.
    // Origin of the mapping is unimportant here.
    let inventory_data: InventoryBackend =
        PullInventoryFileBackend::new(args.inventory_file).into();

    info!("Listen for connections at {}", args.address);

    // If spawning the app via systemd, report that the server is now starting
    let _ = sd_notify::notify(true, &[NotifyState::Ready])
        .map_err(|e| warn!("Could not use sd_notify: {:?}", e));

    let service = hunt_axum_router(get_router(inventory_data).await.unwrap());

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

    let listener = TcpListener::bind(&args.address)
        .await
        .map_err(|e| error!(error = ?e, "could not bind to {}", args.address))?;

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
