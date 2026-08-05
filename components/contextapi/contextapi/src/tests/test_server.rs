// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::future::Future;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::{TcpListener, ToSocketAddrs};
use tokio::sync::oneshot::Sender;
use tracing::{info, info_span};

use crate::api::App;

/// Prepares a server with the given application and address.
///
/// The last argument should be a join handle to the context manager
/// Returns a future representing the server together with its address and a sender
/// for triggering a graceful shutdown.
pub(super) async fn serve_app_with_addr(
    application: App,
    addr: impl ToSocketAddrs,
) -> (
    impl Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>>
    + Send
    + 'static,
    SocketAddr,
    Sender<()>,
) {
    let App {
        router,
        context_manager_join_handle,
        ..
    } = application;
    let listener = TcpListener::bind(addr)
        .await
        .expect("Should be able to bind random port");
    let address = listener
        .local_addr()
        .expect("Should be a valid local address");
    let server = axum::serve(listener, router.into_make_service());

    // Set a graceful shutdown trigger
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    // if the sender is dropped we let the test continue running for up to 180 seconds before the shutdown is triggered
    let sleep = tokio::time::sleep(Duration::from_secs(180));
    let server = server.with_graceful_shutdown(async move {
        info_span!("graceful_shutdown");
        if rx.await.is_err() {
            let _ = sleep.await;
            // TODO: Should we maybe even panic if we get here?
        }
        info!("shutting down the context manager");
        // Cancel the context manager
        context_manager_join_handle.abort();
        let _ = context_manager_join_handle.await;
        info!("context manager shutdown complete");
    });

    let server = async move { server.await.map_err(Into::into) };

    (server, address, tx)
}
