// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

mod persevering_database_interaction;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use chrono::Utc;
use db_interaction::{
    connection::DbFacade,
    models::{
        aliases::MachineId,
        contexts::ContextIdentifier,
        drives::Drive,
        machines::{MachineReservation, MachineReset, MachineState},
    },
};

use db_interaction::schema::contexts as context_schema;
use db_interaction::schema::machine_reservations as machine_reservations_schema;
use db_interaction::schema::machine_resets as machine_resets_schema;
use db_interaction::schema::machines as machines_schema;
use diesel::prelude::*;
use image_api::ImageHandler;
use single_context_api::drives_handler;
use tokio::task::JoinSet;
use tracing::{debug, error, info, instrument, warn};

use crate::{
    NetCtrlClient,
    remote_client::RemoteClient,
    single_context_api::{self, drives_handler::DriveHash},
};

use self::persevering_database_interaction::InteractWithRetriesExt;

/// Attempts to terminate the context. All operations are retried (interleaved with sleeps)
/// until they either succeed or the given duration has elapsed (or some other unrecoverable error occurs).
///
/// Upon failure the context identifier will be returned in which case the caller should schedule a
/// respawn of this context termination task.
#[instrument(skip_all, fields(context_id = %ctx_id.id))]
pub(super) async fn terminate_context(
    ctx_id: ContextIdentifier,
    db_facade: Arc<DbFacade>,
    net_ctrl_client: NetCtrlClient,
    remote_client: RemoteClient,
    timeout: Duration,
    image_handler: ImageHandler,
) -> Result<(), ContextIdentifier> {
    debug!("attempt to terminate context started");
    let start_time = Instant::now();
    let recompute_timeout = move || {
        let elapsed = Instant::now().checked_duration_since(start_time);
        elapsed
            .map(|elapsed| timeout.saturating_sub(elapsed))
            .unwrap_or(timeout)
    };

    // Set the machines belonging to the context (that are not already free) to be marked as resetting and return
    // them from the database
    let Ok(machine_ids) = tokio::time::timeout(recompute_timeout(), async {db_facade
        .write_with_retries(move |conn| {
            conn.transaction::<Vec<MachineId>, diesel::result::Error, _>(|conn| {
                let machine_ids: Vec<MachineId> = MachineReservation::belonging_to(&ctx_id)
                    .inner_join(machines_schema::table)
                    .select(machines_schema::id)
                    .filter(machines_schema::state.ne(MachineState::Free))
                    .load(conn).inspect_err(|_|error!("could not load machines belonging to the context"))?;

                diesel::update(machines_schema::table)
                    .filter(machines_schema::id.eq_any(&machine_ids))
                    .set(machines_schema::state.eq(MachineState::Resetting))
                    .execute(conn).inspect_err(|_|error!("could not mark machines as resetting in the database"))?;

                let started = Utc::now().naive_utc();
                let machine_resets : Vec<MachineReset> = machine_ids.iter().copied().map(|id| {
                    MachineReset {id, started}
                }).collect();

                // Optimistically attempt to insert all machine reset data in a single command and we will handle the error if there are any uniqueness violations.
                // NOTE that applying a `on_conflict_do_nothing` to the entire vec of machine_resets does currently not seem to compile when using sqlite as a backend.
                // I.E. The following example https://docs.diesel.rs/2.2.x/diesel/query_builder/struct.InsertStatement.html#vec-of-records does not seem to work with sqlite.
                if let Err(e) = (&machine_resets).insert_into(machine_resets_schema::table).execute(conn) {
                    // Slow path for the rare case that one or more of these machines already have an entry for their reset metadata. This can happen in the two following cases:
                    // 1) The context api was shutdown before it completed a context termination
                    // 2) The previous attempt to terminate this context timed out (after a presumably long duration) and we are now trying again from the start.
                    warn!(error.dbg = ?e, error.msg = %e, "inserting all machine reset metadata failed. Going to try inserting reset data when necessary for one machine at a time");
                    for machine_reset in machine_resets {
                        let machine_id = machine_reset.id;
                        machine_reset.insert_into(machine_resets_schema::table).on_conflict_do_nothing().execute(conn).inspect_err(|_| error!(%machine_id, "could not insert machine reset metadata into the database even with a do nothing on conflict clause"))?;
                    }
                }
                Ok(machine_ids)
            })
        }).await
        }).await.inspect_err(|e| error!(error.dbg = ?e, error.msg = %e, "timeout occurred while attempting to set machine states to resetting")) else {
            return Err(ctx_id);
        };
    info!("machines successfully marked as resetting");

    // Spawn machine resets
    let mut join_set = JoinSet::new();
    for machine_id in machine_ids {
        let net_ctrl = net_ctrl_client.clone();
        let remote_client = remote_client.clone();
        let db_facade = db_facade.clone();
        join_set.spawn(async move {
            if let Err(e) = machine_ops_lib::machine_reset::reset_machine(machine_id, db_facade, net_ctrl, remote_client).await {
                // If we get an irrecoverable error there is not much we can do. We report the error and let the task sleep until the parent task times out.
                // Maintainers may then fix the corrupted database value in the meantime and the next retry should (hopefully) succeed.
                error!(error.dbg = ?e, "BUG: irrecoverable machine reset error. Please adjust the database. Putting reset task to sleep");
                std::future::pending::<()>().await;
            };
            machine_id
        });
    }

    let mut num_machines_freed = 0;
    let reset_result = tokio::time::timeout(recompute_timeout(), async {
        let mut unexpected_join_error = false;
        while let Some(result) = join_set.join_next().await {
            match result {
                Err(e) => {
                    error!(error.dbg = ?e, error.msg = %e, "unexpexted join error");
                    unexpected_join_error = true;
                }
                Ok(machine_id) => {
                    // Mark the machine as free in the database and make it available for reservation once more.
                    info!(%machine_id, "machine was reset. Going to update database state");
                    db_facade.write_with_retries(move |conn| {
                        conn.immediate_transaction(|conn| {
                            diesel::update(machines_schema::table).filter(machines_schema::id.eq(machine_id)).set(machines_schema::state.eq(MachineState::Free)).execute(conn).inspect_err(|_| error!(%machine_id, "could not mark machine as free in the database"))?;
                            diesel::delete(machine_resets_schema::table).filter(machine_resets_schema::id.eq(machine_id)).execute(conn).inspect_err(|_| error!(%machine_id, "failed to delete machine reset metadata from the database"))?;
                            diesel::delete(machine_reservations_schema::table).filter(machine_reservations_schema::id.eq(machine_id)).execute(conn).inspect_err(|_| error!(%machine_id, "failed to delete machine reservation from the database"))?;
                            Ok(())
                        })
                    }).await;
                    info!(%machine_id, "machine is now free");
                    num_machines_freed += 1;
                }
            }
        }
        if unexpected_join_error {
            Err(ctx_id)
        } else {
            Ok(())
        }
    })
    .await;

    info!(
        num_successful_resets = num_machines_freed,
        "machine reset tasks ended"
    );

    // Note that the tasks in the joinset are aborted once it is dropped,
    reset_result.map_err(|e| {
        error!(error.dbg = ?e, error.msg = %e, "machine reset tasks timed out");
        ctx_id
    })??;

    // This means all machines have been successfully freed!
    // Delete all the drives of the context
    let drives: Vec<Drive> = tokio::time::timeout(recompute_timeout(), async {
        db_facade
            .read_with_retries(move |conn| {
                Drive::belonging_to(&ctx_id)
                    .select(Drive::as_select())
                    .load(conn)
                    .inspect_err(|_| error!("failed to load drives belonging to context"))
            })
            .await
    })
    .await
    .map_err(|e| {
        error!(error.dbg = ?e, error.msg = %e, "attempt to load drive hashes timed out");
        ctx_id
    })?;

    // Now delete the drives from the file system. We don't retry these operations
    // and we also don't want to subtract their duration from our timeout
    let time_before_drive_deletes = Instant::now();
    for drive in drives {
        let Ok(drive_hash) = DriveHash::try_from(drive.id.clone()) else {
            error!("BUG: the stored drive id is not of the expected format");
            continue;
        };
        if let Err(e) =
            drives_handler::handle_delete_drive(image_handler.clone(), &drive_hash).await
        {
            error!(?drive, error = ?e, "error while attempting to delete drive");
        } else {
            info!(?drive, "drive deleted");
        }
    }
    let additional_time = Instant::now()
        .checked_duration_since(time_before_drive_deletes)
        .unwrap_or(Duration::ZERO);
    // Finally we delete the context from the database.

    // Since we have enabled foreign key support and declared deletes to cascade
    // this should also delete all data associated with the context.
    let num_affected_rows = tokio::time::timeout(recompute_timeout() + additional_time, async {
        db_facade.write_with_retries(move |conn| {
            diesel::delete(context_schema::table).filter(context_schema::id.eq(&ctx_id.id))
            .execute(conn)
            .inspect_err(|_|error!("failed to delete context from database"))
        }).await
    } ).await.map_err(|e| {
            error!(error.dbg = ?e, error.msg = %e, "timeout occurred while attempting to delete context from the database");
            ctx_id
        })?;

    info!(num_affected_rows, "context deleted");
    if num_affected_rows == 0 {
        // This indicates a potential bug and/or that some other service (or perhaps directly querying the database through the CLI)
        // is guilty of deleting the context while this task was still running.
        warn!("no errors when deleting context from the database, but no rows were affected");
    }
    Ok(())
}
