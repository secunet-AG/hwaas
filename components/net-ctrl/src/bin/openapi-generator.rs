// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use error_stack::Report;
use error_stack::{Context, Result, ResultExt};
use inventory::InventoryDummyBackend;
use net_ctrl_lib::get_api;
use std::fmt;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
    /// path to file where the OAS is written to
    #[arg(short, long, alias("output-file"))]
    out_file: Option<String>,
}

#[derive(Debug)]
struct OpenApiGeneratorError;

impl fmt::Display for OpenApiGeneratorError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str("Could not generate OAS")
    }
}

impl Context for OpenApiGeneratorError {}

/// Print the OpenAPI specification of the NetCtrl API.
#[tokio::main]
async fn main() -> Result<(), OpenApiGeneratorError> {
    let args: CliArgs = CliArgs::parse();

    let json =
        serde_json::to_value(get_api(InventoryDummyBackend::new(Default::default()).into()).await)
            .map_err(Report::from)
            .change_context(OpenApiGeneratorError)?;
    let json_str = serde_json::to_string_pretty(&json)
        .map_err(Report::from)
        .change_context(OpenApiGeneratorError)?;

    match args.out_file {
        Some(path) => {
            let path = PathBuf::from_str(path.as_str())
                .map_err(Report::from)
                .change_context(OpenApiGeneratorError)?;

            fs::write(path, json_str)
                .map_err(Report::from)
                .change_context(OpenApiGeneratorError)
        }
        _ => {
            println!("{}", json_str);
            Ok(())
        }
    }
}
