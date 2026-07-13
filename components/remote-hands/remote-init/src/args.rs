// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::load_config;
use clap::Parser;
use hunt::HuntBuilder;
use serde::Deserialize;
use std::net::SocketAddr;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct CliArgs {
    /// path to the context config file
    #[arg(short, long, alias("config-file"))]
    pub config_file: String,

    /// the ip address to listen on
    #[arg(short, long, default_value = "127.0.0.1")]
    pub address: String,

    /// the port to listen on
    #[arg(short, long, default_value_t = 8080)]
    pub port: u16,

    /// level of verbosity (could be used several times; e.g. '-vv')
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    #[arg(short, long)]
    pub tokio_console_address: Option<SocketAddr>,
}

impl CliArgs {
    pub fn parse() -> Self {
        <Self as Parser>::parse()
    }

    pub fn load_config<T>(&self) -> T
    where
        T: for<'a> Deserialize<'a>,
    {
        load_config(&self.config_file)
    }

    pub fn address(&self) -> SocketAddr {
        format!("{}:{}", self.address, self.port).parse().unwrap()
    }

    /// Provides a HuntBuilder that has been preconfigured from CLI
    /// args.
    pub fn hunt(&self) -> HuntBuilder {
        HuntBuilder::new()
            .verbosity(self.verbose)
            .enable_otel_layer()
            .append_filters(vec![
                "remote_axum",
                "remote_init",
                "remote_serial",
                "remote_power",
                "remote_usb",
                "remote_auxiliary",
                "hidg",
            ])
            .fallback_name(env!("CARGO_PKG_NAME"))
            .fallback_version(env!("CARGO_PKG_VERSION"))
            .tokio_console_address(self.tokio_console_address)
            .append_filters(vec!["tokio", "tracing"])
    }
}
