// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

mod config;
//mod sim;
mod hw_demand;
mod table;

use crate::config::SimValues;
use crate::hw_demand::HwDemand;
use crate::table::DemandTable;
use clap::Parser;
use error_stack::{Context, Report, ResultExt};
use std::fmt;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
//use crate::sim::sim_main;

/// Program CLI Arguments
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Path to values file
    /// If file does not exist or is empty one with defaults is created
    values: String,

    /// When specified, create/export a CSV file
    #[clap(short, long)]
    csv: Option<PathBuf>,
}

#[derive(Debug)]
struct AppError;
impl fmt::Display for AppError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str("App error")
    }
}

impl Context for AppError {}

fn main() -> Result<(), Report<AppError>> {
    let args: Args = Args::parse();

    let values_file_path = Path::new(&args.values);

    let values_opt: Option<SimValues> = match File::open(values_file_path) {
        Ok(mut file) => {
            let mut values_string = String::new();
            match file.read_to_string(&mut values_string).expect("read file") {
                0 => None,
                _ => {
                    // read successful
                    // values file contend is not empty
                    // try to deserialize now
                    Some(serde_yaml::from_str(values_string.as_str()).expect("valid values"))
                }
            }
        }
        Err(_) => None,
    };

    let values = match values_opt {
        None => {
            let def = SimValues::default();
            File::create(values_file_path)
                .change_context(AppError)?
                .write_all(
                    serde_yaml::to_string(&def)
                        .change_context(AppError)?
                        .as_bytes(),
                )
                .change_context(AppError)?;
            def
        }
        Some(v) => v,
    };

    // TODO: Use [dep-graph](https://crates.io/crates/dep-graph)

    let demand = HwDemand::new(&values).change_context(AppError)?;

    let table = DemandTable::new(&values, &demand);

    table.print().change_context(AppError)?;

    if let Some(path) = args.csv {
        table.export_csv(path).change_context(AppError)?;
    }
    Ok(())
}
