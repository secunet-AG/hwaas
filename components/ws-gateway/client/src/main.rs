// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

//! Websocket Gateway Client
//!
//! This application is supposed to transfer L2-Network Traffic via a WebSocket connection.
//! This WS connection is established between the server and the client part (corresponding to the websocket role).
//! Both parts forward traffic to the Linux kernel networking Stack via AF_PACKET.

#[macro_use]
extern crate tracing;

mod client;
mod error;
mod tap;

use crate::client::WsL2Client;
use crate::error::ClientError;
use clap::{Parser, ValueHint::Url};
use tracing::Level;
use tracing_subscriber::fmt::Subscriber;

/// bridge l2 networks via websocket
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
#[clap(propagate_version = true)]
pub(crate) struct CliArgs {
    /// level of verbosity
    #[clap(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// An URL to establish a websocket connection
    #[clap(short, long, default_value = "ws://127.0.0.1:9002", value_hint(Url))]
    address: String,

    /// initial MTU of the TAP device
    #[clap(short, long, default_value = "1470")]
    mtu: u32,

    /// Name of the linux TAP device to spawn
    dev: String,
}

#[tokio::main]
async fn main() -> Result<(), ClientError> {
    let args: CliArgs = CliArgs::parse();

    init_tracing(args.verbose);

    debug!("connecting");
    {
        WsL2Client::new(args)?.start().await.map(|_| {
            info!("Finished");
        })
    }
    .map_err(|e| {
        error!("{}", e.to_string());
        e
    })
}

pub fn init_tracing(level: u8) {
    // setup tracing (logging)
    tracing::subscriber::set_global_default(get_subscriber(level)).unwrap();
    info!("Max log level: {:?}", get_level(level));
}

pub fn get_subscriber(level: u8) -> Subscriber {
    tracing_subscriber::fmt()
        .with_max_level(get_level(level))
        .finish()
}

pub fn get_level(level: u8) -> Level {
    match level {
        0 => Level::WARN,
        1 => Level::INFO,
        2 => Level::DEBUG,
        _ => Level::TRACE,
    }
}
