// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::machine_data;
use crate::machine_reset;
use chrono::Utc;
use db_interaction::connection::DatabaseInteractionError;
use db_interaction::connection::DbFacade;
use db_interaction::models::aliases::MachineId;
use db_interaction::models::machines::Machine;
use db_interaction::models::machines::MachineReset;
use db_interaction::models::machines::MachineState;
use db_interaction::models::machines::SwitchConnection;
use db_interaction::schema::machine_resets::columns as machine_resets_columns;
use db_interaction::schema::machine_resets::table as machine_resets_table;
use db_interaction::schema::machines::columns as machines_columns;
use db_interaction::schema::machines::table as machines_table;
use db_interaction::schema::switch_connections::table as switch_connections_table;
use db_interaction::schema::switch_ports;
use diesel::prelude::*;
use error_utils::log_err;
use machine_data::MachineData;
use net_ctrl_client_wrapper::NetCtrlClient;
use remote_client::RemoteClient;

use tokio::task::JoinSet;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::instrument;
use tracing::warn;

/// An error that prevented a machine from initializing.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum InitializationError {
    /// The machine was found to be in-use.
    MachineInUse,
    /// A necessary database interaction failed.
    UnsuccessfulDbInteraction,
    /// The reset of the machine did not succeed within the
    /// allocated time frame.
    ResetTimeout,
    /// The reset could not be performed because of malformed
    /// values in the database.
    IrrecoverableResetError,
    /// The machine was successfully upserted and reset, but
    /// the final step of marking it as free did not succeed.
    UnableToMarkAsFree,
    /// Crucial reset metadata associated with the resetting machine
    /// could not be found in the database.
    MissingResetData,
}

/// Errors that may occur when initializing machines.
#[derive(Debug)]
pub struct InitializationErrors {
    pub unsuccessful_initializations: BTreeMap<MachineId, InitializationError>,
    pub join_error_count: u32,
}

impl std::fmt::Display for InitializationErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use InitializationError::*;

        write!(
            f,
            "error(s) occurred when attempting to initializing machines"
        )?;
        for (machine_id, error_cause) in self.unsuccessful_initializations.iter() {
            write!(f, ": could not initialize machine: {}: ", machine_id)?;
            match error_cause {
                MachineInUse => {
                    write!(f, "machine is already in use")?;
                }
                UnsuccessfulDbInteraction => {
                    write!(f, "could not upsert machine data into the database")?;
                }
                ResetTimeout => {
                    write!(f, "the attempt to reset the machine timed out")?;
                }
                UnableToMarkAsFree => {
                    write!(f, "the machine could not be marked as free in the database")?;
                }
                MissingResetData => {
                    write!(
                        f,
                        "BUG: machine reset metadata is missing from the database"
                    )?;
                }
                IrrecoverableResetError => {
                    write!(f, "BUG: machine cannot be reset due to malformed data")?;
                }
            }
        }

        let join_error_count = self.join_error_count;
        if join_error_count > 0 {
            write!(
                f,
                ": {} additional reset tasks could not be joined",
                join_error_count
            )?;
        }
        Ok(())
    }
}

impl std::error::Error for InitializationErrors {}

/// Default timeout for machine reset attempts during initialization.
pub const DEFAULT_MACHINE_RESET_TIMEOUT: u32 = 5 * 60;

/// Options for the [`initialize`](initialize()) function.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "cli", derive(clap::Args))]
pub struct InitializationOptions {
    /// Whether to skip initializing machines that are already marked as
    /// free in the database.
    #[cfg_attr(feature = "cli", arg(long))]
    pub skip_free_machines: bool,
    /// If this is set then the initialization will effectively ignore machines
    /// that are in use rather than erroring.
    #[cfg_attr(feature = "cli", arg(long))]
    pub ignore_in_use: bool,

    /// If this is set then we give up on a machine reset after the specified
    /// time in seconds has passed, otherwise we give up after the [default](self::DEFAULT_MACHINE_RESET_TIMEOUT).
    #[cfg_attr(feature = "cli", arg(long, default_value_t = DEFAULT_MACHINE_RESET_TIMEOUT))]
    pub machine_reset_timeout: u32,
}

impl Default for InitializationOptions {
    fn default() -> Self {
        Self {
            skip_free_machines: false,
            ignore_in_use: false,
            machine_reset_timeout: DEFAULT_MACHINE_RESET_TIMEOUT,
        }
    }
}

/// What action to run next once a machine reset has successfully completed
enum PostResetAction {
    /// The data must now be updated.
    UpdateData,
    /// The machine is now initialized and should be marked as free in the database.
    FreeMachine,
}
/// Initializes all machines and then writes their state as free to the database upon success.
///
/// This function is intended to be called on rare occasions, typically on startup or upon
/// receiving new hardware. Hence not much attention has been placed on optimization.
///
/// # Errors
///
/// We attempt to initialize as many machines as possible rather than erroring out immediately,
/// the exception being if we discover that one or more of the given machines are already in-use
/// prior to initialization. In that case we will error immediately, unless the `ignore_in_use`
/// options parameter is set in which case the machine gets ignored (and a warning is logged).
///
/// If one or more failures are encountered then a summary of everything that went wrong will be
/// returned to the caller.
///
/// ## Definition of In-use
///
/// Machines are considered to be _in-use_ if they are `Reserved` or marked as
/// `Resetting`. In the latter case it is likely that another process, or task is
/// currently administering a reset of the machine since this function places machines
/// in the `Initializing` state and does not update said state until the machine has
/// been successfully reset and prepared (at which point it is set to `Free`).
///
/// # Upsert behaviour
///
/// Existing machines that are not considered `in-use`, get updated and subsequently
/// reset unless the `skip_free_machines` option is passed in which case all existing free
/// machines get ignored by this initialization routine.
///
/// Any new machine (i.e. that does not already exist in the database) will be inserted and
/// reset.
///
/// Inserts and updates take care to change machine states to `Initializing` in order to prevent
/// reservations of said machines before they get fully initialized.
#[instrument(skip_all)]
pub async fn initialize(
    machines: Vec<MachineData>,
    conn: Arc<DbFacade>,
    net_ctrl_client: NetCtrlClient,
    remote_client: RemoteClient,
    options: &InitializationOptions,
) -> Result<(), InitializationErrors> {
    let &InitializationOptions {
        skip_free_machines,
        ignore_in_use,
        machine_reset_timeout,
    } = options;

    let mut errors = InitializationErrors {
        unsuccessful_initializations: BTreeMap::new(),
        join_error_count: 0,
    };
    let mut join_set = JoinSet::new();
    let reset_timeout_duration = Duration::from_secs(machine_reset_timeout.into());

    let mut pending_resets = Vec::new();

    let reset = |machine_id: MachineId, next_task: PostResetAction| {
        let conn = conn.clone();
        let net_ctrl_client = net_ctrl_client.clone();
        let remote_client = remote_client.clone();
        async move {
            let reset_result = tokio::time::timeout(
                reset_timeout_duration,
                machine_reset::reset_machine(machine_id, conn, net_ctrl_client, remote_client),
            )
            .await;
            (machine_id, reset_result, next_task)
        }
    };

    let mut machines_for_updating = HashMap::new();
    for machine_data in machines {
        let id = machine_data.id;
        match preprocess(id, &conn, skip_free_machines, ignore_in_use).await {
            Ok(PreprocessingOutcome::ProceedWithInsert) => {
                // Insert the machine in the database
                if let Err(e) = upsert(machine_data, &conn, false).await {
                    info!(
                        machine_id = id,
                        "failed to insert machine data to the database"
                    );
                    errors.unsuccessful_initializations.insert(id, e);
                    continue;
                };
                // Spawn a reset task for the machine
                info!(
                    machine_id = id,
                    "machine was successfully upserted. Initiating machine reset"
                );
                pending_resets.push(reset(id, PostResetAction::FreeMachine));
            }
            Ok(PreprocessingOutcome::ProceedWithUpdate) => {
                // The machine already exists so we need to reset it with the old data, before updating
                info!(
                    machine_id = id,
                    "going to reset machine prior to machine data upsert"
                );
                pending_resets.push(reset(id, PostResetAction::UpdateData));
                let _ = machines_for_updating.insert(id, machine_data);
            }
            Ok(PreprocessingOutcome::SkipMachine) => {
                info!(machine_id = id, "machine will not be initialized");
                continue;
            }
            Err(InitializationError::MachineInUse) => {
                // We interpret this as a hard error and return immediately
                error!(machine_id = id, "attempt to initialize in-use machine detected. Aborting the entire initialization process");
                let _ = errors
                    .unsuccessful_initializations
                    .insert(id, InitializationError::MachineInUse);
                return Err(errors);
            }
            Err(initialization_error) => {
                error!(
                    machine_id = id,
                    ?initialization_error,
                    "machine upsert failed. Going to process next machine"
                );
                let _ = errors
                    .unsuccessful_initializations
                    .insert(id, initialization_error);
                continue;
            }
        }
    }

    for pending_reset in pending_resets {
        join_set.spawn(pending_reset);
    }

    while let Some(join_result) = join_set.join_next().await {
        if let Ok(reset_result) = join_result.inspect_err(log_err!("unexpected join error")) {
            match reset_result {
                (machine_id, Ok(Ok(_)), next_task) => match next_task {
                    PostResetAction::FreeMachine => {
                        info!(%machine_id, "machine has been successfully reset. Going to mark it as free in the database");

                        if free_machine(machine_id, &conn)
                            .await
                            .inspect_err(log_err!("failed to mark machine as free in the database"))
                            .is_err()
                        {
                            let _ = errors
                                .unsuccessful_initializations
                                .insert(machine_id, InitializationError::UnableToMarkAsFree);
                        }
                    }
                    PostResetAction::UpdateData => {
                        info!(%machine_id, "existing machine has been successfully reset. Going to update data");
                        if let Err(e) = upsert(
                            machines_for_updating
                                .remove(&machine_id)
                                .expect("the machine data should be available"),
                            &conn,
                            true,
                        )
                        .await
                        {
                            let _ = errors.unsuccessful_initializations.insert(machine_id, e);
                            error!(%machine_id, "failed to update machine data");
                            continue;
                        }
                        join_set.spawn(reset(machine_id, PostResetAction::FreeMachine));
                    }
                },
                (machine_id, Ok(err), _) => {
                    error!(%machine_id, error.dbg = ?err, "BUG: an irrecoverable error was discovered when attempting to reset the machine");
                    let _ = errors
                        .unsuccessful_initializations
                        .insert(machine_id, InitializationError::IrrecoverableResetError);
                }
                (machine_id, Err(timeout), _) => {
                    error!(%machine_id, ?timeout, "machine reset attempt timed out");
                    let _ = errors
                        .unsuccessful_initializations
                        .insert(machine_id, InitializationError::ResetTimeout);
                }
            }
        } else {
            // Note that if a join error occurred then we already logged it using `inspect_err` above.
            errors.join_error_count += 1;
        }
    }
    if (!errors.unsuccessful_initializations.is_empty()) || (errors.join_error_count > 0) {
        Err(errors)
    } else {
        Ok(())
    }
}

/// Mark the machine as free in the database.
#[instrument(skip(db_facade))]
async fn free_machine(
    machine_id: MachineId,
    db_facade: &DbFacade,
) -> Result<(), DatabaseInteractionError> {
    db_facade
        .execute_on_current_thread(|conn| {
            conn.immediate_transaction(|conn| {
                diesel::update(machines_table)
                    .set(machines_columns::state.eq(MachineState::Free))
                    .filter(machines_columns::id.eq(machine_id))
                    .execute(conn)
                    .inspect_err(|_| error!("failed to set machine state to free"))?;

                diesel::delete(machine_resets_table)
                    .filter(machine_resets_columns::id.eq(machine_id))
                    .execute(conn)
                    .inspect_err(|_| error!("failed to delete entry in machine_resets table"))?;
                Ok(())
            })
        })
        .await
}

/// The outcome returned by [`preprocess`],
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum PreprocessingOutcome {
    /// Skip processing this machine.
    SkipMachine,
    /// The machine exist. It should be updated.
    ProceedWithUpdate,
    /// The machine does not already exist. Insert it!
    ProceedWithInsert,
}

/// Accesses the database and determines how to proceed
///
/// If the machine does not already exist it returns [`PreprocessingOutcome::ProceedWithInsert`],
/// if it does exist then this will result in one of the following outcomes:
///
/// - If the machine is determined to be in-use and the `ignore_in_use` flag is set we get
/// [`PreprocessingOutcome::SkipMachine`].
/// - If the machine is determined to be in-use and the `ignore_in_use` flag is `false` we
/// get an error.
/// - If the machine is free and the `skip_free_machines` flag is set we get
/// [`PreprocessingOutcome::SkipMachine`].
/// - If the machine is free and the `skip_free_machines` flag is `false` then
/// we get [`PreprocessingOutcome::ProceedWithUpdate`]
/// - If the machine is deactivated the machine then we get
/// [`PreprocessingOutcome::ProceedWithUpdate`]
#[instrument(skip(conn))]
async fn preprocess(
    machine_id: MachineId,
    conn: &DbFacade,
    skip_free_machines: bool,
    ignore_in_use: bool,
) -> Result<PreprocessingOutcome, InitializationError> {
    let mut initialization_error = None;
    let mut evaluation = PreprocessingOutcome::SkipMachine;
    let transaction_result = conn
            .execute_on_current_thread(|conn| {
                conn.exclusive_transaction(|conn| {
                    let machine_state_lookup: Option<MachineState> = machines_table
                        .select(
                            machines_columns::state
                        )
                        .filter(machines_columns::id.eq(machine_id))
                        .first(conn)
                        .optional()
                        .inspect_err(|_| error!("machine lookup returned an error"))?;
                    if machine_state_lookup.is_none() {
                        // This means that the machine does not already exist in the database and it should be inserted
                        evaluation = PreprocessingOutcome::ProceedWithInsert;
                    }

                    if let Some(state) = machine_state_lookup {
                        // If the machine already exists in the database we need some special handling
                        // depending on the parameters
                        evaluation = match state {
                            MachineState::Deactivated => PreprocessingOutcome::ProceedWithUpdate,
                            MachineState::Free if skip_free_machines => {
                                debug!("machine is currently free. No changes will be made to this machine!");
                                PreprocessingOutcome::SkipMachine
                            },
                            MachineState::Free => {
                                debug!("machine is currently free. Going to initialize machine");
                                PreprocessingOutcome::ProceedWithUpdate
                            },
                            MachineState::Initializing => {
                                info!("machine is marked as initializing in the database. We expect it to be safe to retry this operation. Proceeding with initialization");
                                PreprocessingOutcome::ProceedWithUpdate
                                }
                            other_state if ignore_in_use => {
                                warn!(state = ?other_state, "machine is currently in use. No changes will be made to this machine by us!");
                                PreprocessingOutcome::SkipMachine
                            },
                            other_state => {
                                error!(state = ?other_state, "machine is currently in use. Aborting!");
                                initialization_error = Some(InitializationError::MachineInUse);
                                return Err(diesel::result::Error::RollbackTransaction);
                            }
                        }
                    };
                    if (evaluation == PreprocessingOutcome::SkipMachine) || (evaluation == PreprocessingOutcome::ProceedWithInsert) {
                        // We complete the transaction here before performing any
                        // writes.
                        Ok(())
                    } else  {
                            // Set machine state to initializing because we will run a reset before updating machine data
                            diesel::update(machines_table).set(machines_columns::state.eq(MachineState::Initializing)).filter(machines_columns::id.eq(machine_id)).execute(conn).inspect_err(|_| error!("unable to set machine state to initializing"))?;
                            let started = Utc::now().naive_utc();
                            let machine_reset = MachineReset {id: machine_id, started};
                            machine_reset.insert_into(machine_resets_table).on_conflict(machine_resets_columns::id).do_update().set(machine_resets_columns::started.eq(started)).execute(conn).inspect_err(|_| error!("unable to upsert machine reset metadata"))?;
                            Ok(())
                            }
                        })
                    }).await;
    if let Some(err) = initialization_error {
        return Err(err);
    }
    if let Err(e) = transaction_result {
        log_err!("machine preprocessing transaction failed")(&e);
        return Err(InitializationError::UnsuccessfulDbInteraction);
    }
    Ok(evaluation)
}
/// Upsert the given machine in the database
///
/// If this function succeeds it is guaranteed that the
/// machine's state **will be `Initializing`.
#[instrument(skip(conn))]
async fn upsert(
    machine_data: MachineData,
    conn: &DbFacade,
    already_exists: bool,
) -> Result<(), InitializationError> {
    let transaction_result = conn
        .execute_on_current_thread(|conn| {
            conn.exclusive_transaction(|conn| {
                let machine_id = machine_data.id;
                let MachineData {
                    id,
                    platform,
                    remote_usb,
                    remote_power,
                    switch_connections,
                    remote_serial,
                    remote_auxiliary,
                } = machine_data;
                let machine = Machine {
                    id,
                    platform,
                    remote_usb: remote_usb.into(),
                    remote_power: remote_power.into(),
                    remote_serial: remote_serial.map(Into::into),
                    remote_auxiliary: remote_auxiliary.map(Into::into),
                    state: MachineState::Initializing,
                };
                let machine_reset = MachineReset {
                    id,
                    started: Utc::now().naive_utc(),
                };

                if already_exists {
                    // We simply delete the machine, then re-insert it. Note that due to cascades all
                    // related machine data should also get deleted.
                    // Furthermore we recall that this is happening inside a transaction hence if anything fails
                    // then we don't alter the state of the database prior to this attempt.
                    // Todo: error reporting
                    diesel::delete(machines_table)
                        .filter(machines_columns::id.eq(machine_id))
                        .execute(conn)?;
                }

                // Now (re)-insert the machine
                {
                    info!("about to upsert machine in the database");
                    (&machine)
                        .insert_into(machines_table)
                        .execute(conn)
                        .inspect_err(|_| {
                            error!("error when attempting to insert machine into the database")
                        })?;

                    // Insert the machine's resetting metadata
                    machine_reset
                        .insert_into(machine_resets_table)
                        .execute(conn)
                        .inspect_err(|_| {
                            error!(
                            "error when attempting to insert machine reset metadata to the database"
                        )
                        })?;
                    // Upsert the machine's switch ports to obtain their ids. We do this in a loop as database access is very fast in sqlite.
                    for (interface, switch_port) in switch_connections.iter() {
                        debug!(?switch_port, "about to upsert switch port");
                        switch_port
                            .insert_into(switch_ports::table)
                            .on_conflict_do_nothing()
                            .execute(conn)
                            .inspect_err(|_| {
                                error!(?switch_port, "error when attempting to upsert switch port")
                            })?;
                        // Read the switch port id from the database now that we know it's there.
                        // Note that we could not have used a returning clause as it returns an empty
                        // set in the case of conflicting upserts in sqlite.
                        let switch_port_id: i32 = {
                            use switch_ports::columns::{id, port, switch};
                            switch_ports::table
                                .select(id)
                                .filter(
                                    port.eq(switch_port.port.clone())
                                        .and(switch.eq(&switch_port.switch)),
                                )
                                .get_result(conn)
                                .inspect_err(|_| {
                                    error!(
                                        ?switch_port,
                                        "unable to retrieve switch port id from the database"
                                    )
                                })
                        }?;
                        let switch_connection = SwitchConnection {
                            id: switch_port_id,
                            interface: interface.clone(),
                            machine_id,
                        };
                        debug!(?switch_connection, "About to insert switch connection");
                        diesel::insert_into(switch_connections_table)
                            .values(&switch_connection)
                            .execute(conn)
                            .inspect_err(|_| {
                                error!(
                                    ?switch_connection,
                                    "error when attempting to insert switch connection"
                                );
                            })?;
                    }
                }

                Ok(())
            })
        })
        .await;

    if transaction_result
        .inspect_err(log_err!("failed to commit machine upsert transaction"))
        .is_err()
    {
        return Err(InitializationError::UnsuccessfulDbInteraction);
    };

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        sync::atomic::{AtomicBool, Ordering},
    };

    use self::machine_data::MachineNetworkInterface;

    use super::*;
    use chrono::DateTime;
    use db_interaction::{
        models::machines::{Machine, SwitchPort, SwitchPortValue},
        test_utils::TestDb,
    };
    use tracing_test::traced_test;
    use wiremock::{matchers::any, Mock, MockServer, ResponseTemplate};

    struct RemoteMocksPerMachine {
        remote_usb: MockServer,
        remote_power: MockServer,
        remote_auxiliary: MockServer,
        remote_serial: MockServer,
    }

    /// Assertions to check in regard to the machine related data
    /// in the database after an initialization attempt.
    enum DbMachineAssertions {
        /// Assert that the database consists of the
        /// machines in the test setup with their states
        /// set to free.
        MachinesFromTestSetup,
        /// Assert that the database consists of exactly
        /// these machine data, state pairs.
        Equals(Vec<(MachineData, MachineState)>),
    }

    /// Attempts to initialize the machines described in the given `test_setup`
    /// with the given initialization `options`.
    ///
    /// A `result_inspector`
    /// can be passed to add additional checks on the expected result of the
    /// initialization routine.
    ///
    /// The `post_initialization_db_assertions` parameter describes the
    /// machine data (and machine states) that should exist in the database
    /// once the initialization routine has completed.
    async fn initialize_and_assert<ResultInspector: FnMut(Result<(), InitializationErrors>)>(
        test_setup: &mut TestSetup,
        options: &InitializationOptions,
        mut result_inspector: ResultInspector,
        post_initialization_db_assertions: DbMachineAssertions,
    ) {
        let result = initialize(
            test_setup.machines.clone(),
            test_setup.db_facade.clone(),
            test_setup.net_ctrl_client.clone(),
            test_setup.remote_client.clone(),
            options,
        )
        .await;
        dbg!(&result);
        result_inspector(result);
        // Assert the expected database state
        {
            // Load all machines together with their related data.
            let machines: Vec<Machine> = machines_table
                .select(Machine::as_select())
                .load(&mut test_setup.db.conn)
                .unwrap();

            let switch_connections: Vec<SwitchConnection> =
                db_interaction::schema::switch_connections::table
                    .select(SwitchConnection::as_select())
                    .load(&mut test_setup.db.conn)
                    .unwrap();

            let switch_ports: Vec<SwitchPort> = db_interaction::schema::switch_ports::table
                .select(SwitchPort::as_select())
                .load(&mut test_setup.db.conn)
                .unwrap();

            let mut switch_ports_per_machine = HashMap::new();
            for switch_connection in switch_connections {
                let SwitchConnection {
                    id,
                    interface,
                    machine_id,
                } = switch_connection;
                let port: SwitchPortValue = switch_ports
                    .iter()
                    .find(|port| port.id == id)
                    .unwrap()
                    .clone()
                    .into();

                let switch_ports_for_machine: &mut HashMap<
                    MachineNetworkInterface,
                    SwitchPortValue,
                > = switch_ports_per_machine.entry(machine_id).or_default();
                switch_ports_for_machine.insert(interface, port);
            }

            let data_per_machine: Vec<(MachineData, MachineState)> = machines
                .into_iter()
                .map(|machine| {
                    let Machine {
                        id,
                        platform,
                        remote_usb,
                        remote_power,
                        remote_serial,
                        remote_auxiliary,
                        state,
                        ..
                    } = machine;
                    let switch_connections =
                        switch_ports_per_machine.remove(&id).unwrap_or_default();

                    let machine_data = MachineData {
                        id,
                        platform,
                        remote_usb: remote_usb.try_into().unwrap(),
                        remote_power: remote_power.try_into().unwrap(),
                        switch_connections,
                        remote_serial: remote_serial
                            .map(|serial_addr| serial_addr.try_into().unwrap()),
                        remote_auxiliary: remote_auxiliary.map(|addr| addr.try_into().unwrap()),
                    };
                    (machine_data, state)
                })
                .collect();

            dbg!(&data_per_machine);
            let contains = |machine_data, machine_state| {
                data_per_machine.iter().any(|machine_data_from_db| {
                    (&machine_data_from_db.0 == machine_data)
                        && (machine_data_from_db.1 == machine_state)
                })
            };
            use DbMachineAssertions::*;
            match post_initialization_db_assertions {
                MachinesFromTestSetup => {
                    let machines = &test_setup.machines;
                    assert_eq!(machines.len(), data_per_machine.len());
                    for machine in machines {
                        assert!(
                            contains(machine, MachineState::Free),
                            "Machine {} definition for state free not found in store",
                            machine.id
                        );
                    }
                }
                Equals(machine_data_state_pairs) => {
                    dbg!(&machine_data_state_pairs);
                    assert_eq!(machine_data_state_pairs.len(), data_per_machine.len());
                    for (machine_data, state) in &machine_data_state_pairs {
                        assert!(contains(machine_data, *state));
                    }
                }
            }
        }
    }

    async fn test_machine(
        id: i32,
        port_id: i32,
        default_mock_behaviour: bool,
    ) -> (MachineData, RemoteMocksPerMachine) {
        let mocks = RemoteMocksPerMachine {
            remote_usb: MockServer::start().await,
            remote_power: MockServer::start().await,
            remote_serial: MockServer::start().await,
            remote_auxiliary: MockServer::start().await,
        };
        if default_mock_behaviour {
            let mock_gen = || Mock::given(any()).respond_with(ResponseTemplate::new(200));
            mocks.remote_usb.register(mock_gen()).await;
            mocks.remote_power.register(mock_gen()).await;
            mocks.remote_serial.register(mock_gen()).await;
            mocks.remote_auxiliary.register(mock_gen()).await;
        }
        let data = MachineData {
            id,
            remote_usb: format!("{}/usb", mocks.remote_usb.uri())
                .try_into()
                .unwrap(),
            remote_power: format!("{}/power", mocks.remote_power.uri())
                .try_into()
                .unwrap(),
            platform: String::from("SR630"),
            switch_connections: [(
                String::from("lan1"),
                SwitchPortValue {
                    switch: String::from("switch1"),
                    port: port_id.to_string(),
                },
            )]
            .into_iter()
            .collect(),
            remote_serial: Some(
                format!("{}/serial", mocks.remote_serial.uri())
                    .try_into()
                    .unwrap(),
            ),
            remote_auxiliary: Some(
                format!("{}/auxiliaries", mocks.remote_auxiliary.uri())
                    .try_into()
                    .unwrap(),
            ),
        };
        (data, mocks)
    }

    struct TestSetup {
        machines: Vec<MachineData>,
        mocks_per_machine: Vec<RemoteMocksPerMachine>,
        db: TestDb,
        db_facade: Arc<DbFacade>,
        net_ctrl_client: NetCtrlClient,
        remote_client: RemoteClient,
        net_ctrl_mock: MockServer,
    }

    impl TestSetup {
        /// Creates a new test setup: This includes an empty database and a set
        /// of machine descriptions that we may want to pass to the initialization
        /// routine.
        async fn new(num_machines: u8, default_mock_behaviour: bool) -> Self {
            const DB_POOL_SIZE: u32 = 8;
            let db = TestDb::spawn();
            let db_facade = DbFacade::new(db.file.as_path().to_str().unwrap(), DB_POOL_SIZE)
                .await
                .unwrap();
            let db_facade = Arc::new(db_facade);
            let net_ctrl_mock = MockServer::start().await;
            if default_mock_behaviour {
                net_ctrl_mock
                    .register(Mock::given(any()).respond_with(ResponseTemplate::new(200)))
                    .await;
            }
            let net_ctrl_base_path = net_ctrl_mock.uri().to_owned();
            let net_ctrl_client = NetCtrlClient::new(net_ctrl_base_path);
            let remote_client = RemoteClient::default();

            let mut machines = Vec::with_capacity(num_machines.into());
            let mut mocks = Vec::with_capacity(num_machines.into());
            for id in 1..=num_machines {
                let (machine, mock_servers) =
                    test_machine(i32::from(id), i32::from(id), default_mock_behaviour).await;
                machines.push(machine);
                mocks.push(mock_servers)
            }
            Self {
                machines,
                mocks_per_machine: mocks,
                db,
                db_facade,
                net_ctrl_client,
                remote_client,
                net_ctrl_mock,
            }
        }

        /// Moves the mocks per machine out of the TestSetup.
        ///
        /// This is useful to avoid issues with the borrow checker if you need
        /// to hold mock references over a call to  [`initialize_and_assert`].
        fn take_mocks_per_machine(&mut self) -> Vec<RemoteMocksPerMachine> {
            std::mem::take(&mut self.mocks_per_machine)
        }
    }

    // Check that initialize works under the simplest conditions.
    #[tokio::test]
    #[traced_test]
    async fn initializing_one_machine_to_clean_db_works() {
        let mut test_setup = TestSetup::new(1, true).await;
        // let machine_ids = test_setup.machine_ids();
        initialize_and_assert(
            &mut test_setup,
            &Default::default(),
            |result| assert!(result.is_ok()),
            DbMachineAssertions::MachinesFromTestSetup,
        )
        .await;
    }

    #[tokio::test]
    #[traced_test]
    async fn initializing_many_machines_to_clean_db_works() {
        let mut test_setup = TestSetup::new(8, true).await;
        // let machine_ids = test_setup.machine_ids();

        // Let's also set some options
        let options = InitializationOptions {
            skip_free_machines: true,
            ignore_in_use: false,
            machine_reset_timeout: 60,
        };

        initialize_and_assert(
            &mut test_setup,
            &options,
            |result| assert!(result.is_ok()),
            DbMachineAssertions::MachinesFromTestSetup,
        )
        .await;
    }

    // Check that a free machine gets updated and reset if
    // `skip_free` is `false` (the default)
    #[tokio::test]
    #[traced_test]
    async fn upsert_free_machine() {
        let mut test_setup = TestSetup::new(1, true).await;
        let MachineData {
            id,
            platform,
            remote_usb,
            remote_power,
            remote_serial,
            remote_auxiliary,
            ..
        } = test_setup.machines.first().unwrap().clone();
        // Insert a machine marked as `Free` with a platform not matching the
        // one to be specified in the initialization process.
        let machine = Machine {
            id,
            platform: format!("not {}", &platform),
            remote_usb: remote_usb.into(),
            state: MachineState::Free,
            remote_power: remote_power.into(),
            remote_serial: remote_serial.map(TryInto::try_into).transpose().unwrap(),
            remote_auxiliary: remote_auxiliary.map(TryInto::try_into).transpose().unwrap(),
        };
        machine
            .insert_into(machines_table)
            .execute(&mut test_setup.db.conn)
            .unwrap();

        initialize_and_assert(
            &mut test_setup,
            &Default::default(),
            |result| assert!(result.is_ok()),
            DbMachineAssertions::MachinesFromTestSetup,
        )
        .await;
    }

    // Check that if a switch port already exists in the database then this does not make
    // machine initialization fail.
    #[tokio::test]
    #[traced_test]
    async fn preexisting_ports_do_not_make_initialization_fail() {
        let mut test_setup = TestSetup::new(8, true).await;

        let switch_port = test_setup
            .machines
            .first()
            .unwrap()
            .switch_connections
            .values()
            .next()
            .unwrap();
        switch_port
            .insert_into(switch_ports::table)
            .execute(&mut test_setup.db.conn)
            .unwrap();

        // let machine_ids = test_setup.machine_ids();
        initialize_and_assert(
            &mut test_setup,
            &Default::default(),
            |result| assert!(result.is_ok()),
            DbMachineAssertions::MachinesFromTestSetup,
        )
        .await;
    }

    // Check that initialization succeeds after a while even if the
    // external components (remote hands services and/or net controller)
    // are flaky (i.e. need a couple of retries before they succeed).
    #[tokio::test]
    #[traced_test]
    async fn initialization_works_with_flaky_external_components() {
        let mut test_setup = TestSetup::new(2, false).await;
        // Setup flaky mocks that always succeed on the second attempt.
        let mock_gen = || {
            let has_failed = AtomicBool::new(false);
            Mock::given(any())
                .respond_with(move |_req: &wiremock::Request| {
                    if !has_failed.load(Ordering::Acquire) {
                        has_failed.store(true, Ordering::Release);
                        ResponseTemplate::new(500)
                    } else {
                        ResponseTemplate::new(200)
                    }
                })
                .expect(2..)
        };
        test_setup.net_ctrl_mock.register(mock_gen()).await;

        for machine_mocks in &test_setup.mocks_per_machine {
            let RemoteMocksPerMachine {
                remote_usb,
                remote_power,
                remote_auxiliary,
                remote_serial,
                ..
            } = machine_mocks;

            remote_usb.register(mock_gen()).await;
            remote_power.register(mock_gen()).await;
            remote_auxiliary.register(mock_gen()).await;
            remote_serial.register(mock_gen()).await;
        }

        initialize_and_assert(
            &mut test_setup,
            &Default::default(),
            |result| assert!(result.is_ok()),
            DbMachineAssertions::MachinesFromTestSetup,
        )
        .await;
    }

    // Check that we can retry initialization of a machine whose state is `Initializing`.
    #[tokio::test]
    #[traced_test]
    async fn machine_initialization_retry() {
        let mut test_setup = TestSetup::new(1, true).await;
        // Insert the first machine prior to running the initialization routine.
        // The inserted machine will be in the `Initializing` state.
        let initialization_machine_data = test_setup.machines.first().unwrap().clone();
        // We also insert the machine with data different from what is stated in the initialization description.
        let mut original_machine_data = initialization_machine_data.clone();
        original_machine_data.platform =
            String::from("A platform not specified in the initialization description");
        // Also no serials and auxiliaries in the original
        original_machine_data.remote_serial = None;
        original_machine_data.remote_auxiliary = None;
        original_machine_data.switch_connections = HashMap::new();
        let MachineData {
            id,
            platform,
            remote_usb,
            remote_power,
            remote_serial,
            remote_auxiliary,
            ..
        } = original_machine_data.clone();
        let machine = Machine {
            id,
            platform,
            remote_usb: remote_usb.into(),
            remote_power: remote_power.into(),
            remote_serial: remote_serial.map(Into::into),
            remote_auxiliary: remote_auxiliary.map(Into::into),
            state: MachineState::Initializing,
        };
        // The previous reset started around 2023
        let reset_started = DateTime::from_timestamp(60 * 60 * 24 * 365 * 53, 400).unwrap();
        let machine_reset = MachineReset {
            id,
            started: reset_started.naive_utc(),
        };
        machine
            .insert_into(machines_table)
            .execute(&mut test_setup.db.conn)
            .unwrap();
        machine_reset
            .insert_into(db_interaction::schema::machine_resets::table)
            .execute(&mut test_setup.db.conn)
            .unwrap();

        // We declare that machines that are considered in-use to be ignored.
        let options = InitializationOptions {
            ignore_in_use: true,
            ..Default::default()
        };

        initialize_and_assert(
            &mut test_setup,
            &options,
            |result| assert!(result.is_ok()),
            DbMachineAssertions::MachinesFromTestSetup,
        )
        .await;
    }

    // Check that if we attempt to initialize a machine which is already in use, then
    // we get an error and the machine is not updated. This is the expected behaviour
    // when using default options.
    #[tokio::test]
    #[traced_test]
    async fn error_on_initializing_in_use_machine() {
        for state in [MachineState::Reserved, MachineState::Resetting] {
            let mut test_setup = TestSetup::new(1, true).await;
            // Let us insert a machine to the database with the same id as the one described in the test data, but
            // with some different attributes.
            let mut original_machine_data = test_setup.machines.first().unwrap().clone();
            original_machine_data.platform =
                "A platform not specified in the initialization description".to_string();
            original_machine_data.switch_connections = Default::default();
            original_machine_data.remote_serial = Default::default();
            original_machine_data.remote_auxiliary = Default::default();
            let MachineData {
                id,
                platform,
                remote_usb,
                remote_power,
                remote_serial,
                remote_auxiliary,
                ..
            } = original_machine_data.clone();

            let machine = Machine {
                id,
                platform,
                remote_usb: remote_usb.into(),
                remote_power: remote_power.into(),
                remote_serial: remote_serial.map(Into::into),
                remote_auxiliary: remote_auxiliary.map(Into::into),
                state,
            };

            diesel::insert_into(machines_table)
                .values(machine)
                .execute(&mut test_setup.db.conn)
                .unwrap();

            if state == MachineState::Resetting {
                let machine_reset = MachineReset {
                    id,
                    started: Utc::now().naive_utc(),
                };
                machine_reset
                    .insert_into(machine_resets_table)
                    .execute(&mut test_setup.db.conn)
                    .unwrap();
            }
            let expected_machines_in_db = vec![(original_machine_data, state)];

            initialize_and_assert(
                &mut test_setup,
                &Default::default(),
                |result| {
                    assert_eq!(
                        result.unwrap_err().unsuccessful_initializations,
                        BTreeMap::from_iter(std::iter::once((
                            id,
                            InitializationError::MachineInUse,
                        ))),
                    );
                },
                DbMachineAssertions::Equals(expected_machines_in_db),
            )
            .await;
        }
    }

    // Check that the skip_free_machines and ignore_in_use options work as expected.
    //
    // We attempt to initialize 5 machines where four of them are already in
    //the database  with state equals Deactivated, Free, Reserved and Resetting
    //respectively. When the ignore options are set only the deactivated and
    //non-existing machines get initialized, the other machines do not get
    // altered.
    #[tokio::test]
    #[traced_test]
    async fn skip_and_ignore_options_work() {
        let options = InitializationOptions {
            skip_free_machines: true,
            ignore_in_use: true,
            machine_reset_timeout: 60,
        };

        let mut test_setup = TestSetup::new(5, true).await;

        // Insert the first four machines into the database prior to machine initialization. We make some
        // modifications so we are sure they are not equal to the machines described in the initialization
        // configuration.
        let mut pre_inserted_machine_data = Vec::new();
        for (state, machine_data) in [
            MachineState::Deactivated,
            MachineState::Free,
            MachineState::Reserved,
            MachineState::Resetting,
        ]
        .into_iter()
        .zip(test_setup.machines.iter())
        {
            let mut original_machine_data = machine_data.clone();
            original_machine_data.platform = String::from("Another platform");
            original_machine_data.switch_connections = Default::default();
            original_machine_data.remote_serial = Default::default();
            original_machine_data.remote_auxiliary = Default::default();
            let MachineData {
                id,
                platform,
                remote_usb,
                remote_power,
                remote_serial,
                remote_auxiliary,
                ..
            } = original_machine_data.clone();
            let machine = Machine {
                id,
                platform,
                remote_usb: remote_usb.into(),
                remote_power: remote_power.into(),
                remote_serial: remote_serial.map(Into::into),
                remote_auxiliary: remote_auxiliary.map(Into::into),
                state,
            };
            machine
                .insert_into(machines_table)
                .execute(&mut test_setup.db.conn)
                .unwrap();
            if state == MachineState::Resetting {
                let machine_reset = MachineReset {
                    id,
                    started: Utc::now().naive_utc(),
                };
                machine_reset
                    .insert_into(machine_resets_table)
                    .execute(&mut test_setup.db.conn)
                    .unwrap();
            }
            pre_inserted_machine_data.push((original_machine_data, state));
        }

        // Now when we run the initialization routine, the deactivated machine and the non-preexisting machine
        // should get initialized. Everything else should be left as is.
        let mut expected_post_initialization_data = pre_inserted_machine_data;
        // The first entry we inserted had state set to Deactivated. We need to update our expectation as
        // the machine should be initialized (leading to `Free` as its state).
        let (machine_data, state) = expected_post_initialization_data.get_mut(0).unwrap();
        assert_eq!(*state, MachineState::Deactivated);
        *state = MachineState::Free;
        *machine_data = test_setup.machines.first().unwrap().clone();

        // Also recall that there is one more machine we didn't insert prior to initialization. We also expect
        // this to get initialized.
        expected_post_initialization_data.push((
            test_setup.machines.last().unwrap().clone(),
            MachineState::Free,
        ));

        initialize_and_assert(
            &mut test_setup,
            &options,
            |result| assert!(result.is_ok()),
            DbMachineAssertions::Equals(expected_post_initialization_data),
        )
        .await;

        // Check that the mocks corresponding to the initialized machines were indeed called.
        let first_machine_mocks = test_setup.mocks_per_machine.first().unwrap();
        let last_machine_mocks = test_setup.mocks_per_machine.last().unwrap();
        // The first machine already existed so should get reset twice (once prior to upsert and once after)
        // the last machine is new so should only get reset once. Note that in the case of the first machine we
        // started out with no serial ports and no auxiliaries hence those only get reset once.
        let expected_resets = [2, 1];
        for (mocks, num_resets) in [first_machine_mocks, last_machine_mocks]
            .iter()
            .zip(expected_resets)
        {
            assert_eq!(
                mocks.remote_power.received_requests().await.unwrap().len(),
                num_resets
            );
            assert_eq!(
                mocks.remote_usb.received_requests().await.unwrap().len(),
                num_resets
            );
            assert_eq!(
                mocks.remote_serial.received_requests().await.unwrap().len(),
                1
            );
            assert_eq!(
                mocks
                    .remote_auxiliary
                    .received_requests()
                    .await
                    .unwrap()
                    .len(),
                1
            );
        }
        // Check that no requests were sent for the machines that were skipped or ignored.
        for mocks in &test_setup.mocks_per_machine[1..4] {
            assert!(mocks
                .remote_power
                .received_requests()
                .await
                .unwrap_or_default()
                .is_empty());
            assert!(mocks
                .remote_usb
                .received_requests()
                .await
                .unwrap_or_default()
                .is_empty());
            assert!(mocks
                .remote_serial
                .received_requests()
                .await
                .unwrap_or_default()
                .is_empty());
            assert!(mocks
                .remote_auxiliary
                .received_requests()
                .await
                .unwrap_or_default()
                .is_empty());
        }
    }

    // Check that if a machine fails to reset within the given timeout value
    // then the initialization process will return an error.
    //
    // We also assert that the failure of one machine to reset and thus complete
    // its initialization does not affect other machines initializations.
    #[tokio::test]
    #[traced_test]
    async fn machine_reset_timeout_behavior() {
        // Generates a failing mock with priority 2 (highest is 1).
        let failing_mock = || {
            Mock::given(any())
                .respond_with(ResponseTemplate::new(500))
                .with_priority(2)
                .expect(1..)
        };

        // Generates a succeeding mock with the highest priority.
        let succeeding_mock = || {
            Mock::given(any())
                .respond_with(ResponseTemplate::new(200))
                .with_priority(1)
        };
        // Create a test setup where we describe two machines.
        let mut test_setup = TestSetup::new(2, false).await;

        // For simplicity we remove the described switch ports from the second machine.
        test_setup.machines[1].switch_connections = Default::default();

        let net_ctrl_mock_guard = test_setup
            .net_ctrl_mock
            .register_as_scoped(
                Mock::given(any())
                    .respond_with(ResponseTemplate::new(200))
                    .with_priority(3),
            )
            .await;

        let mocks_per_machine = test_setup.take_mocks_per_machine();
        // We will update the behavior of these mocks for the first machine over several iterations.
        let first_machine_mocks = setup_successful_remote_mocks(&mocks_per_machine[0]).await;
        // We will mostly just let the second machine succeed initialization.
        let _ = setup_successful_remote_mocks(&mocks_per_machine[1]).await;
        // Run one initialization attempt per remote component of the first machine and make the respective mock
        // server fail.
        let first_machine_id = test_setup.machines[0].id;

        // Make resets of machines give up after a second. Note that we expect the reset never to
        // succeed for the first machine.
        let options = InitializationOptions {
            machine_reset_timeout: 1,
            ..Default::default()
        };
        let result_inspector = |result: Result<(), InitializationErrors>| {
            assert_eq!(
                result.unwrap_err().unsuccessful_initializations,
                BTreeMap::from_iter(std::iter::once((
                    first_machine_id,
                    InitializationError::ResetTimeout
                )))
            );
        };
        let expected_post_initialization_db_state: Vec<(MachineData, MachineState)> = test_setup
            .machines
            .iter()
            .cloned()
            .zip([MachineState::Initializing, MachineState::Free])
            .collect();
        for (iteration, mock_server) in first_machine_mocks.iter().enumerate() {
            let _guard = mock_server
                .register_as_scoped(
                    failing_mock().named(format!("mock-component-number-{}", iteration)),
                )
                .await;

            initialize_and_assert(
                &mut test_setup,
                &options,
                result_inspector,
                DbMachineAssertions::Equals(expected_post_initialization_db_state.clone()),
            )
            .await;
            // Register a succeeding mock with higher priority. This step is actually necessary
            // even though the failing mock guard is dropped. TODO: Why?
            mock_server.register(succeeding_mock()).await;
        }

        // We now make the net controller fail all requests.
        drop(net_ctrl_mock_guard);
        test_setup.net_ctrl_mock.register(failing_mock()).await;
        // Recall as the second machine has no switch connections it can still be (re-initialized),
        // while the first should fail this time as well.
        initialize_and_assert(
            &mut test_setup,
            &options,
            result_inspector,
            DbMachineAssertions::Equals(expected_post_initialization_db_state),
        )
        .await;
    }

    // Check that it is possible to initialize a machine that does not have services configured
    // for its optional peripheral classes
    #[tokio::test]
    #[traced_test]
    async fn initialize_without_optional_peripheral_classes() {
        let mut test_setup = TestSetup::new(1, true).await;
        for machine in &mut test_setup.machines {
            machine.remote_serial = None;
            machine.remote_auxiliary = None;
            // The following is not relevant for the test, but ensures compilation breaks if
            // we make any of these values optional in the future. Thus we may adapt the test.
            // This works because Option<T> does not implement Display.
            print!("{}", machine.remote_usb);
            print!("{}", machine.remote_power);
        }
        initialize_and_assert(
            &mut test_setup,
            &Default::default(),
            |result| assert!(result.is_ok()),
            DbMachineAssertions::MachinesFromTestSetup,
        )
        .await;

        // Check that requests sent to mocks during startup are as expected. The services that do not have addresses in the machine data should not have received anything
        // while the others must at least have received one request.
        for RemoteMocksPerMachine {
            remote_usb,
            remote_power,
            remote_auxiliary,
            remote_serial,
        } in &test_setup.mocks_per_machine
        {
            assert!(remote_serial
                .received_requests()
                .await
                .unwrap_or_default()
                .is_empty());
            assert!(remote_auxiliary
                .received_requests()
                .await
                .unwrap_or_default()
                .is_empty());
            // The following services are configured so should at least have received one request
            assert!(!remote_usb
                .received_requests()
                .await
                .unwrap_or_default()
                .is_empty());
            assert!(!remote_power
                .received_requests()
                .await
                .unwrap_or_default()
                .is_empty());
        }
    }

    /// Sets up succeeding mocks with priority 3 (highest is 1) and returns the remote mocks per machine
    /// as a flat vec of mock servers.
    async fn setup_successful_remote_mocks(
        remote_mocks_for_machine: &RemoteMocksPerMachine,
    ) -> Vec<&MockServer> {
        let mut mock_list = Vec::new();
        let mock_gen = || {
            Mock::given(any())
                .respond_with(ResponseTemplate::new(200))
                .with_priority(3)
        };

        let power_mock_server = &remote_mocks_for_machine.remote_power;
        power_mock_server.register(mock_gen()).await;
        mock_list.push(power_mock_server);

        let usb_mock_server = &remote_mocks_for_machine.remote_usb;
        usb_mock_server.register(mock_gen()).await;
        mock_list.push(usb_mock_server);

        let serial_mock_server = &remote_mocks_for_machine.remote_serial;
        serial_mock_server.register(mock_gen()).await;
        mock_list.push(serial_mock_server);

        let auxiliary_mock_server = &remote_mocks_for_machine.remote_auxiliary;
        auxiliary_mock_server.register(mock_gen()).await;
        mock_list.push(auxiliary_mock_server);

        mock_list
    }
}
