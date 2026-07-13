// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use connection_handler::SwitchMapping;
use error_stack::{Context, Report, Result, ResultExt};
use schemars::schema_for;
use std::path::PathBuf;
use std::str::FromStr;
use std::{fmt, fs};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
    /// path to file where the OAS is written to
    #[arg(short, long, alias("output-file"))]
    out_file: Option<String>,
}

#[derive(Debug)]
struct ConfigSchemaGeneratorError;

impl fmt::Display for ConfigSchemaGeneratorError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str("Could not generate config schema")
    }
}

impl Context for ConfigSchemaGeneratorError {}

fn main() -> Result<(), ConfigSchemaGeneratorError> {
    let args: CliArgs = CliArgs::parse();

    let schema = schema_for!(SwitchMapping);
    let schema_str = serde_json::to_string_pretty(&schema)
        .map_err(Report::from)
        .change_context(ConfigSchemaGeneratorError)?;

    match args.out_file {
        Some(path) => {
            let path = PathBuf::from_str(path.as_str())
                .map_err(Report::from)
                .change_context(ConfigSchemaGeneratorError)?;

            fs::write(path, schema_str)
                .map_err(Report::from)
                .change_context(ConfigSchemaGeneratorError)
        }
        _ => {
            println!("{}", schema_str);
            Ok(())
        }
    }
}
