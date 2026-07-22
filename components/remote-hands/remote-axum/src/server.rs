// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use axum::Router;
use hunt::{hunt_axum_router, Hunt};
use sd_notify::NotifyState;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::{debug, info, warn};

async fn wait_for_signal() {
    use tokio::signal::unix::{signal, SignalKind};

    let mut sigint = signal(SignalKind::interrupt()).unwrap();
    let mut sigterm = signal(SignalKind::terminate()).unwrap();

    tokio::select! {
        _ = sigint.recv() => debug!("Received SIGINT"),
        _ = sigterm.recv() => debug!("Received SIGTERM"),
    };
}

pub async fn run_axum_server(
    address: SocketAddr,
    router: Router,
    hunt: Hunt,
) -> Result<(), std::io::Error> {
    info!("listen for connections at {}", address);
    let listener = TcpListener::bind(address).await.unwrap();

    // If spawning the app via systemd, report that the server is now starting
    let _ = sd_notify::notify(true, &[NotifyState::Ready])
        .map_err(|e| warn!("could not use sd_notify: {:?}", e));

    let service = hunt_axum_router(router);

    axum::serve(listener, service)
        .with_graceful_shutdown(wait_for_signal())
        .await
        .unwrap();

    info!("shutting down...");
    drop(hunt);

    Ok(())
}
