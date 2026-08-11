// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use axum::http::Method;
use clap::Parser;
use error_stack::{Report, ResultExt};
use hunt::{HuntBuilder, hunt_axum_router};
use sd_notify::NotifyState;
use std::fs;
use std::net::SocketAddr;
use std::process::exit;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info};

use context_api_lib::ContextApiConfig;
use context_api_lib::api::App;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
    /// path to the context config file
    #[arg(short, long, alias("config-file"))]
    config_file: String,

    /// the ip address to listen on
    #[arg(short, long, default_value = "127.0.0.1")]
    address: String,

    /// the port to listen on
    #[arg(short, long, default_value_t = 8080)]
    port: u16,

    /// level of verbosity (could be used several times; e.g. '-vv')
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    #[arg(short, long)]
    tokio_console_address: Option<SocketAddr>,
}

#[derive(Debug)]
struct ApplicationError;

impl std::fmt::Display for ApplicationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the context api terminated because of an unexpected error"
        )
    }
}
impl std::error::Error for ApplicationError {}

#[tokio::main]
async fn main() -> Result<(), error_stack::Report<ApplicationError>> {
    let args: CliArgs = CliArgs::parse();

    let hunt = HuntBuilder::new()
        .verbosity(args.verbose)
        .enable_otel_layer()
        .append_filters(vec![
            "contextapi",
            "context_api_lib",
            "ws_proxy_gateway_lib",
            "net_ctrl_client",
            "image_api",
            "network_id_store",
            "machine_ops_lib",
            "remote_client",
        ])
        .fallback_name(env!("CARGO_PKG_NAME"))
        .fallback_version(env!("CARGO_PKG_VERSION"))
        .tokio_console_address(args.tokio_console_address)
        .build();

    let config_file = fs::read_to_string(args.config_file.clone())
        .attach_printable_lazy(|| format!("Unable to read file: {}", args.config_file))
        .change_context(ApplicationError)?;
    let config = serde_json::from_str::<ContextApiConfig>(&config_file)
        .attach_printable("The provided config file was not of the expected JSON format")
        .change_context(ApplicationError)?;
    let App {
        router,
        context_manager_join_handle,
        ..
    } = App::prepare(config)
        .await
        .change_context(ApplicationError)?;

    let address = format!("{}:{}", args.address, args.port)
        .parse::<SocketAddr>()
        .attach_printable_lazy(|| {
            format!(
                "Could not parse a valid socket address from the given address: {} and port: {}",
                args.address, args.port
            )
        })
        .change_context(ApplicationError)?;

    info!("listen for connections at {}", address);

    // channel to trigger server shutdown
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    tokio::spawn(async {
        // override the default SIGINT signal handler for the runtime of this app
        // SIGTERM does not have any special handler. therefore, it has the default behavior.
        tokio::signal::ctrl_c().await.unwrap();
        let _ = tx
            .send(())
            .map_err(|_| error!("Failed to initiate graceful shutdown"));

        // exit immediately when 2nd SIGINT is received
        tokio::signal::ctrl_c().await.unwrap();
        exit(1);
    });

    // Enable CORS
    let cors = CorsLayer::very_permissive()
        // allow specific methods when accessing the resource
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::DELETE,
            Method::PUT,
            Method::PATCH,
        ])
        // allow requests from any origin
        .allow_origin(Any)
        // disabel credentials as it conflicts with the any origin; enabled by 'very_permissive'
        .allow_credentials(false);

    let router = router.layer(cors);

    // If spawning the app via systemd, report that the server is now starting
    let _ = sd_notify::notify(true, &[NotifyState::Ready])
        .map_err(|e| error!(error.dbg = ?e, error.msg = %e, "could not use sd_notify:"));

    let service = hunt_axum_router(router);

    let listener = TcpListener::bind(&address)
        .await
        .attach_printable_lazy(|| format!("Could not bind tcp listener to address: {}", address))
        .change_context(ApplicationError)?;
    let graceful = axum::serve(listener, service).with_graceful_shutdown(async {
        // wait for SIGINT signal to initiate shutdown
        rx.await.ok();
        info!("shutting down...");
        // Abort the context manager and wait until the abort is completed.
        info!("aborting context manager task");
        context_manager_join_handle.abort();
        // Note that we expect an error as we aborted the task ourselves, but log if there was a panic (this should not be the case).
        let _ = context_manager_join_handle.await.inspect_err(|e| {
            if e.is_panic() {
                error!(error.dbg = ?e, error.msg = %e, "the context manager task panicked");
            }
        });
        info!("context manager task has been cancelled");
        drop(hunt);
    });

    if let Err(e) = graceful.await {
        error!(error.msg = %e, error.dbg = ?e, "error while attempting to gracefully shut down the server");
        return Err(Report::new(ApplicationError)
            .attach_printable(format!("could not shut down the server gracefully: {}", e)));
    }

    Ok(())
}
