// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use clap::Args;
use machine_ops_lib::initialization::InitializationOptions;
use std::path::PathBuf;

#[derive(Clone, Debug, Args)]
pub(crate) struct MachineInitializationArgs {
    /// path to the machine declarations file
    #[arg(short, long)]
    pub(crate) machines_file: PathBuf,

    /// path to the context api configuration file
    #[arg(short, long)]
    pub(crate) context_api_config: PathBuf,
    /// level of verbosity (could be used several times; e.g. '-vv')
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub(crate) verbose: u8,

    #[clap(flatten)]
    pub(crate) initialization_options: InitializationOptions,
}
