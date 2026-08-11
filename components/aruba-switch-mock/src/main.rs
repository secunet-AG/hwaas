// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

mod app_state;
mod credential_cookie_name;
mod handlers;
mod main_router;
mod middleware_auth;
mod middleware_route_call_stats;
mod middleware_tracing;

use crate::main_router::get_router;
use clap::Parser;
use std::fs::File;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{Level, info};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{Layer, filter, fmt};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
    /// the socket address to listen on
    #[arg(default_value = "127.0.0.1:8080")]
    address: SocketAddr,

    /// if specified logging messages are written to this file (additionally to stdout)
    #[arg(short, long)]
    log_file: Option<PathBuf>,

    /// level of verbosity (could be used several times; e.g. '-vv')
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

#[tokio::main]
async fn main() {
    let args = CliArgs::parse();

    let level = match args.verbose {
        0 => Level::WARN,
        1 => Level::INFO,
        2 => Level::DEBUG,
        _ => Level::TRACE,
    };

    setup_tracing(level, args.log_file);

    // build our application with a route
    let app = get_router();

    // run it
    info!("listening on {}", args.address);
    axum::Server::bind(&args.address).serve(app).await.unwrap();
}

fn setup_tracing(level: Level, log_file_path: Option<PathBuf>) {
    let filter = filter::Targets::default()
        .with_default(level)
        .with_target("net_ctrl_server", level)
        .with_target("hyper", Level::WARN);

    let debug_log = match &log_file_path {
        Some(p) => {
            let file = File::create(p).expect("created log file");
            Some(
                fmt::layer()
                    .with_writer(Arc::new(file))
                    .json()
                    .with_filter(filter.clone()),
            )
        }
        None => None,
    };

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_filter(filter))
        .with(debug_log)
        .init();

    info!("Log level is set to: {:?}", level);
    if let Some(p) = log_file_path {
        info!("Logfile is written to: {:?}", p)
    }
}
