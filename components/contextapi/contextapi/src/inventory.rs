// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, sync::Arc};

use aide::{
    axum::{ApiRouter, routing::get_with},
    transform::TransformOperation,
};
use axum::{
    Json,
    extract::{FromRef, State},
    http::StatusCode,
};

use chrono::{NaiveDateTime, Utc};
use context_data_structures::machine_properties::MachineProperties;
use db_interaction::{
    connection::DbFacade,
    models::{
        aliases::MachineId,
        machines::{Machine, MachineState},
    },
};
use diesel::prelude::*;
use error_utils::log_then_replace_err;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::error;

#[derive(Clone)]
pub(crate) struct InventoryApiState {
    db_facade: Arc<DbFacade>,
}

impl InventoryApiState {
    pub(crate) fn new(db_facade: Arc<DbFacade>) -> Self {
        Self { db_facade }
    }
}

/// TODO: write tests
///
/// Assert that:
///
/// - Context IDS are not leaked.
/// - When a machine gets reserved (via context reservation) it then
///   becomes observable when this endpoint is called.
/// - Similarly when a machine is freed (because of a context
///   deletion) it then becomes observable when this endpoint is
///   called.
pub(crate) fn inventory_api_router<S>() -> ApiRouter<S>
where
    S: Send + Sync + Clone + 'static,
    InventoryApiState: FromRef<S>,
{
    ApiRouter::<S>::new().api_route(
        "/",
        get_with(handle_get_inventory, api_method_doc_get_inventory),
    )
}
fn api_method_doc_get_inventory(op: TransformOperation) -> TransformOperation {
    op.description("Returns metadata about all known machines")
        .summary("get machine inventory")
        .response_with::<200, Json<Vec<InventoryEntry>>, _>(|op| {
            op.description("Returns metadata about machines")
        })
        .response_with::<500, String, _>(|op| op.description("Internal error reason"))
}

#[derive(Clone, JsonSchema, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// The possible states of a Machine in the Inventory.
enum InventoryEntryState {
    /// The Machine can be reserved
    #[default]
    Free,
    /// The Machine has been reserved
    Reserved,
    /// The Machine is no longer reserved
    /// and is in the process of being freed.
    ///
    /// This might take a while.
    ReservationEnded,
}

/// A machine with reserved/available state, and metadata properties.
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
struct InventoryEntry {
    state: InventoryEntryState,
    properties: MachineProperties,
    /// The estimated number of seconds remaining of the machine reservation.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(skip_serializing_if = "Option::is_none")]
    estimated_reservation_duration: Option<u64>,
    /// The machine's unique identifier.
    machine_id: u16,
}

/// Handler to obtain metadata about all the machines in the machine store
async fn handle_get_inventory(
    State(state): State<InventoryApiState>,
) -> Result<Json<Vec<InventoryEntry>>, (StatusCode, &'static str)> {
    use db_interaction::schema;
    // Load all machines
    let machines: Vec<Machine> = state
        .db_facade
        .spawn_call(|conn| {
            schema::machines::table
                .select(Machine::as_select())
                .load(conn)
        })
        .await
        .map_err(log_then_replace_err!(
            "failed to load machines from the database",
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Could not load machines from the database",
            )
        ))?;

    // Find the ids of the machines that are reserved
    let reserved_machine_ids: Vec<MachineId> = machines
        .iter()
        .filter_map(|machine| match machine.state {
            MachineState::Reserved => Some(machine.id),
            _ => None,
        })
        .collect();

    // Load the context timeout per reserved machine. We can do this by joining the machine
    // machine_reservations table with the context_lifetimes table (on the context id column).
    //
    // NOTE: We cannot simply just load this table with no filter, as machine's that are in the
    // process of being reset may still exist in the machine_reservations table until the
    // entire context gets deleted.
    let timeout_per_reserved_machine: Vec<(MachineId, NaiveDateTime)> = state
        .db_facade
        .spawn_call(|conn| {
            schema::machine_reservations::table
                .inner_join(
                    schema::context_lifetimes::table.on(schema::machine_reservations::context_id
                        .eq(schema::context_lifetimes::context_id)),
                )
                .select((
                    schema::machine_reservations::id,
                    schema::context_lifetimes::timeout,
                ))
                .filter(schema::machine_reservations::id.eq_any(reserved_machine_ids))
                .load(conn)
        })
        .await
        .map_err(log_then_replace_err!(
            "failed to load reserved machine id, timeout pairs from the database",
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Could not load machine reservation durations from the database",
            )
        ))?;

    // Insert all loaded machines into a hashmap for simple O(1) lookups.
    // We also keep an entry for the remaining reservation times (`Option<u64>`).
    let mut machines = machines
        .into_iter()
        .map(|machine| {
            let id = machine.id;
            (id, (machine, None))
        })
        .collect::<HashMap<MachineId, (Machine, Option<u64>)>>();

    // Loop through the loaded timeouts per machine and adjust the `machines` hashmap
    // accordingly
    for (id, date_time) in timeout_per_reserved_machine {
        let timeout = date_time.and_utc();
        let remaining = timeout - Utc::now();
        let remaining_seconds = std::cmp::max(0, remaining.num_seconds()) as u64;
        let Some((_, seconds_until_free)) = machines.get_mut(&id) else {
            error!(
                machine_id = id,
                "BUG: machine lookup failed, but the machine should have previously been loaded"
            );
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Missing machines detected. This could be a bug which we encourage you to report to the HWaaS maintainers",
            ));
        };
        *seconds_until_free = Some(remaining_seconds);
    }

    // Create the vector of user facing machine inventory entries from the `machines` map.
    let mut entries: Vec<InventoryEntry> = machines
        .into_iter()
        .filter_map(|(_, (machine, timeout))| {
            let state = match machine.state {
                MachineState::Free => Some(InventoryEntryState::Free),
                MachineState::Reserved => Some(InventoryEntryState::Reserved),
                MachineState::Resetting => Some(InventoryEntryState::ReservationEnded),
                _ => None,
            }?;
            let platform = machine.platform;
            let id = machine.id;
            // We expect the id to be canonically representable by a u16. Indeed we have a JSON schema dictating that we set it so in the Nix configuration.
            // IIRC we cannot (easily) enforce this directly in the type system as SQLite does not have this fine grained numeric types.
            let id = u16::try_from(id).inspect_err(|e| error!(machine_id = id, error.dbg = ?e, error.msg = %e, "BUG: invalid machine id encountered. Skipping machine from inventory listing.")).ok()?;
            Some(InventoryEntry {
                state,
                properties: MachineProperties { platform },
                estimated_reservation_duration: timeout,
                machine_id: id
            })
        })
        .collect();

    // Sort the entries by machine id
    // to obtain a deterministic output.
    entries.sort_by_key(|entry| entry.machine_id);

    Ok(Json::from(entries))
}
