// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::{fs::File, sync::Arc};

use anyhow::Context;
use clap::{Parser, Subcommand};
use context_api_lib::ContextApiConfig;
use db_interaction::connection::{DbFacade, ProvisionalDbConnection};
use diesel::SqliteConnection;
use initialization_cli_args::MachineInitializationArgs;
use machine_ops_lib::{machine_data::MachineData, network_identifiers::upsert_network_ids};
use net_ctrl_client_wrapper::NetCtrlClient;
use network_ids_cli_args::InsertNetworkIdsArgs;
use remote_client::RemoteClient;
use schemars::{schema_for, JsonSchema};
use serde::Deserialize;
use tracing::{debug, error, Level};
use tracing_subscriber::{fmt::writer::MakeWriterExt, FmtSubscriber};
mod initialization_cli_args;
mod network_ids_cli_args;

#[derive(Parser, Debug)]
#[command(author, version, about = "Machine operations for HWaaS maintainers", long_about = None)]
struct CliArgs {
    #[command(subcommand)]
    commands: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Initialize machines for use in the Context API
    InitializeMachines(MachineInitialization),
    /// Insert network ids for use in the Context API
    InsertNetworkIds(NetworkIdInsertion),
}

#[derive(Debug, Parser)]
struct MachineInitialization {
    #[command(subcommand)]
    command: MachineInitializationCommand,
}

#[derive(Debug, Subcommand, Clone)]
enum MachineInitializationCommand {
    /// Initialize machines specified in a JSON file
    Run(MachineInitializationArgs),
    /// Print the JSON schema of the machines-file
    PrintSchema,
}

#[derive(Debug, Parser)]
struct NetworkIdInsertion {
    #[command(subcommand)]
    command: NetworkIdInsertionCommand,
}

#[derive(Debug, Subcommand, Clone)]
enum NetworkIdInsertionCommand {
    /// Insert the given network identifiers to the database
    Run(InsertNetworkIdsArgs),
    /// Print the JSON schema of the network-ids-file
    PrintSchema,
}

#[derive(Deserialize, JsonSchema)]
#[serde(transparent)]
#[schemars(transparent)]
struct ParsedMachinesFile(Vec<MachineData>);

#[derive(Deserialize, JsonSchema)]
#[serde(transparent)]
#[schemars(transparent)]
struct ParsedNetworkIdsFile(Vec<i16>);

#[tokio::main(flavor = "current_thread")]
pub async fn main() -> anyhow::Result<()> {
    let CliArgs { commands } = CliArgs::parse();

    match commands {
        Commands::InitializeMachines(cmd) => handle_machine_init_command(cmd.command).await,
        Commands::InsertNetworkIds(cmd) => handle_insert_network_ids_command(cmd.command),
    }
}

// Print the schema of the given type to stdout formatted as json.
fn print_schema<T: JsonSchema>() -> anyhow::Result<()> {
    let schema = schema_for!(T);
    let schema =
        serde_json::to_string_pretty(&schema).context("Failed to serialize schema to JSON")?;
    println!("{}", schema);
    Ok(())
}

#[tracing::instrument]
fn establish_db_connection(database: String) -> anyhow::Result<SqliteConnection> {
    debug!("establishing database connection");
    let conn = ProvisionalDbConnection::new(&database)
        .inspect_err(
            |e| error!(%database, error.dbg = ?e, "could not establish database connection"),
        )
        .with_context(|| {
            format!(
                "Could not establish connection to sqlite database: {}",
                &database
            )
        })?
        .configured()
        .inspect_err(|e| error!(error.dbg = ?e, "could not configure database connection"))
        .context("Could not configure database connection")?;

    debug!("established database connection");
    Ok(conn)
}

async fn handle_machine_init_command(command: MachineInitializationCommand) -> anyhow::Result<()> {
    match command {
        MachineInitializationCommand::PrintSchema => print_schema::<ParsedMachinesFile>(),
        MachineInitializationCommand::Run(args) => {
            // The size of the database pool when initializing machines.
            const DB_POOL_SIZE: u32 = 1;
            let MachineInitializationArgs {
                machines_file,
                context_api_config,
                verbose,
                initialization_options,
            } = args;

            let ctx_api_config: ContextApiConfig = serde_json::from_reader(
                File::open(context_api_config)
                    .with_context(|| "Could not open context api config file")?,
            )
            .with_context(|| "Invalid context api config")?;

            let ContextApiConfig {
                net_ctrl_base_path,
                db_file_path,
                ..
            } = ctx_api_config;
            let verbosity: Option<Level> = match verbose {
                0 => {
                    // We interpret this as no tracing
                    None
                }
                1 => Some(Level::ERROR),
                2 => Some(Level::WARN),
                3 => Some(Level::INFO),
                4 => Some(Level::DEBUG),
                _ => Some(Level::TRACE),
            };

            if let Some(level) = verbosity {
                let stderr = std::io::stderr.with_max_level(Level::WARN);
                let subscriber = FmtSubscriber::builder()
                    .with_max_level(level)
                    .map_writer(move |w| stderr.or_else(w))
                    .finish();

                tracing::subscriber::set_global_default(subscriber)
                    .context("Failed to set up logging")?;
            }

            let machines_file_contents = std::fs::read_to_string(machines_file.as_path())
                .with_context(|| {
                    format!(
                        "Failed to read contents from {}",
                        machines_file
                            .as_path()
                            .to_str()
                            .unwrap_or("the provided file path")
                    )
                })?;

            let ParsedMachinesFile(machines_for_initialization) =
                serde_json::from_str(&machines_file_contents).inspect_err(|e| {
                    error!(error.dbg = ?e, file = ?machines_file, "unable to deserialize machine data from file")
                }).with_context(|| format!("Failed to parse the machines-file {}", machines_file.to_str().unwrap_or("")))?;

            let conn = DbFacade::new(&db_file_path, DB_POOL_SIZE)
                .await
                .inspect_err(
                    |e| error!(error.dbg = ?e, error.msg = %e, "unable to create database facade"),
                )
                .with_context(|| {
                    format!(
                        "Failed to setup database connection. Database-file: {}",
                        &db_file_path
                    )
                })?;
            let net_ctrl_client = NetCtrlClient::new(net_ctrl_base_path);
            let remote_client = RemoteClient::default();

            machine_ops_lib::initialization::initialize(
                machines_for_initialization,
                Arc::new(conn),
                net_ctrl_client,
                remote_client,
                &initialization_options,
            )
            .await
            .inspect_err(|e| error!(error.dbg = ?e, "machine initialization errors"))
            .context("Something went wrong when attempting to initialize machine(s)")
        }
    }
}

fn handle_insert_network_ids_command(command: NetworkIdInsertionCommand) -> anyhow::Result<()> {
    match command {
        NetworkIdInsertionCommand::PrintSchema => print_schema::<ParsedNetworkIdsFile>(),
        NetworkIdInsertionCommand::Run(InsertNetworkIdsArgs {
            network_ids_file,
            database,
        }) => {
            let mut conn = establish_db_connection(database)?;

            let network_id_file_contents = std::fs::read_to_string(network_ids_file.as_path())
                .with_context(|| format!("Failed to read: {:?} to string", &network_ids_file))?;

            let ParsedNetworkIdsFile(network_ids) =
                serde_json::from_str(&network_id_file_contents).inspect_err(|e| {
                    error!(error.dbg = ?e, file = ?network_ids_file, "unable to deserialize network ids from file")
                }).with_context(|| format!("Failed to parse the network-ids-file {}", network_ids_file.to_str().unwrap_or("")))?;

            upsert_network_ids(&network_ids, &mut conn).map_err(Into::into)
        }
    }
}
