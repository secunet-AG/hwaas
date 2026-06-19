// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use bytesize::ByteSize;
use clap::Parser;
use context_api_lib::api::get_api;
use context_api_lib::{ContextApiConfig, WsGatewaySettings};
use error_stack::{Context, Report, Result, ResultExt};
use hunt::HuntBuilder;
use image_api::ImageApiSettings;
use std::fmt;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
    /// path to the context config file
    #[arg(short, long, alias("remote-hands-openapi-spec"))]
    remote_oas_paths: Vec<String>,

    /// path to file where the OAS is written to
    #[arg(short, long, alias("output-file"))]
    out_file: Option<String>,

    /// level of verbosity (could be used several times; e.g. '-vv')
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

#[derive(Debug)]
struct OpenApiGeneratorError;

impl fmt::Display for OpenApiGeneratorError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str("Could not generate OAS")
    }
}

impl Context for OpenApiGeneratorError {}

/// Print the OpenAPI specification of the Context API.
fn main() -> Result<(), OpenApiGeneratorError> {
    let args: CliArgs = CliArgs::parse();

    let _hunt = HuntBuilder::new()
        .verbosity(args.verbose)
        .append_filters(vec![
            "contextapi",
            "context_api_lib",
            "ws_proxy_gateway_lib",
            "net_ctrl_client",
            "image_api",
            "network_id_store",
        ])
        .fallback_name(env!("CARGO_PKG_NAME"))
        .fallback_version(env!("CARGO_PKG_VERSION"))
        .build();

    #[allow(clippy::single_range_in_vec_init)]
    let conf = ContextApiConfig {
        net_ctrl_base_path: "https://localhost/".to_string(),
        image_api_settings: Default::default(),
        network_gateway: WsGatewaySettings {
            ws_gateway_url: "".to_string(),
        },
        remote_oas_paths: args.remote_oas_paths,
        remote_max_request_size: Default::default(),
        request_timeouts: Default::default(),
        context_lifetime: Default::default(),
        context_max_lifetime: Default::default(),
        // We are not connecting to a database when generating the OAS
        db_file_path: String::new(),
        max_db_connections: Default::default(),
    };

    // TODO: Use extracted schemas by uncommenting
    // This results in a huge change in our OAS and should be reviewes
    // in a separate MR
    // aide::gen::extract_schemas(true);

    let json = serde_json::to_value(get_api(conf))
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
