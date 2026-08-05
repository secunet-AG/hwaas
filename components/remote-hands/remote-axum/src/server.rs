// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use axum::Router;
use hunt::{Hunt, hunt_axum_router};
use sd_notify::NotifyState;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{debug, info, warn};

async fn wait_for_signal() {
    use tokio::signal::unix::{SignalKind, signal};

    let mut sigint = signal(SignalKind::interrupt()).unwrap();
    let mut sigterm = signal(SignalKind::terminate()).unwrap();

    tokio::select! {
        _ = sigint.recv() => debug!("Received SIGINT"),
        _ = sigterm.recv() => debug!("Received SIGTERM"),
    }
}

pub type CancelHook = Arc<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

pub async fn run_axum_server(
    address: SocketAddr,
    router: Router,
    hunt: Hunt,
) -> Result<(), std::io::Error> {
    run_axum_server_with_cleanup(address, router, hunt, None).await
}

pub async fn run_axum_server_with_cleanup(
    address: SocketAddr,
    router: Router,
    hunt: Hunt,
    cleanup: Option<CancelHook>,
) -> Result<(), std::io::Error> {
    info!("listen for connections at {}", address);
    let listener = TcpListener::bind(address).await?;

    // If spawning the app via systemd, report that the server is now starting
    let _ = sd_notify::notify(true, &[NotifyState::Ready])
        .map_err(|e| warn!("could not use sd_notify: {:?}", e));

    let service = hunt_axum_router(router);

    axum::serve(listener, service)
        .with_graceful_shutdown(wait_for_signal())
        .await?;

    if let Some(cb) = cleanup {
        cb().await;
    }

    info!("shutting down...");
    drop(hunt);

    Ok(())
}
