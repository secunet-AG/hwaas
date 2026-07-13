// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use clap::Args;
use std::path::PathBuf;

#[derive(Clone, Debug, Args)]
pub(crate) struct InsertNetworkIdsArgs {
    /// path to the file containing the network ids for insertion.
    #[arg(short, long)]
    pub(crate) network_ids_file: PathBuf,

    /// The path to the sqlite database file.
    #[arg(short, long)]
    pub(crate) database: String,
}
