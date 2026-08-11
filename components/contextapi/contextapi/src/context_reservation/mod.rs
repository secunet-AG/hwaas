// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use aide::{
    axum::{ApiRouter, routing::post_with},
    transform::TransformOperation,
};
use axum::{
    Json,
    extract::{FromRef, State},
    http::StatusCode,
};
use chrono::Utc;
use context_data_structures::{aliases::ContextId, rsd::Rsd};
use db_interaction::models::{aliases::MachineId, machines::MachineState as ModelMachineState};
use db_interaction::models::{
    context_id::ContextIdBytes,
    contexts::{ContextIdentifier, ContextLifetime},
    machines::MachineReservation,
};
use db_interaction::schema::context_lifetimes as context_lifetimes_schema;
use db_interaction::schema::contexts as contexts_schema;
use db_interaction::schema::machine_reservations as machine_reservations_schema;
use db_interaction::schema::machines as machines_schema;
use db_interaction::{connection::DbFacade, models::machines::Machine as ModelMachine};
use diesel::prelude::*;
use error_utils::{log_err, log_then_replace_err};
use tokio::sync::Semaphore;
use tokio::sync::mpsc::Sender;
use tokio::time::Duration;
use tracing::{error, error_span};

use crate::{ContextApiConfig, context_manager::ContextManagerMessage};

use self::reservation_logic::RsdResolutionError;

mod reservation_logic;

#[derive(Clone)]
pub(crate) struct ContextCreationApiState {
    /// This semaphore always only holds one permit and it is initialized at startup.
    ///
    /// While database transactions ensure atomicity of context reservations, we
    /// prefer to queue up context reservation attempts to avoid the complexity of
    /// writing code that handles transaction errors.
    ///
    /// # IMPORTANT
    ///
    /// Do not add more permits to this semaphore!
    semaphore: Arc<Semaphore>,
    ctx_manager_tx: Sender<ContextManagerMessage>,
    config: Arc<ContextApiConfig>,
    db_facade: Arc<DbFacade>,
}

impl ContextCreationApiState {
    pub(crate) fn new(
        db_facade: Arc<DbFacade>,
        config: Arc<ContextApiConfig>,
        ctx_manager_tx: Sender<ContextManagerMessage>,
    ) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(1)),
            ctx_manager_tx,
            config,
            db_facade,
        }
    }
}

pub(crate) fn context_creation_api_router<S>() -> ApiRouter<S>
where
    S: Send + Sync + Clone + 'static,
    ContextCreationApiState: FromRef<S>,
{
    ApiRouter::new().api_route(
        "/",
        post_with(handle_context_reservation, api_method_doc_post),
    )
}

fn api_method_doc_post(op: TransformOperation) -> TransformOperation {
    op.description(
        "Reserves machines satisfying the given RSD. \
        A context identifier is returned which is used to \
        access the context consisting of the reserved machines.",
    )
    .summary("Reserve a context")
    .response::<200, String>()
    .response_with::<422, String, _>(|op| {
        op.description("The RSD contained an unsatisfiable constraint")
    })
}

/// Reserve machines matching the `rsd` for a new context whose
/// identifier is returned.
#[tracing::instrument(skip(dependencies))]
async fn handle_context_reservation(
    State(dependencies): State<ContextCreationApiState>,
    Json(rsd): Json<Rsd>,
) -> Result<String, (StatusCode, String)> {
    if rsd.machines.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            String::from("Empty resource setup descriptions are not permitted"),
        ));
    }

    let guard = dependencies.semaphore.acquire().await.inspect_err(|e| error!(error.dbg = ?e, error.msg = %e, "BUG: unexpected acquire error: the semaphore used for context reservation should not be closed"));
    if guard.is_err() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            String::from("A bug was encountered. Please inform the HWaaS maintainers"),
        ));
    }
    // Simply load all machines. We can optimize on this in later iterations.
    let machines = dependencies
        .db_facade
        .spawn_call(|conn| {
            db_interaction::schema::machines::table
                .select(ModelMachine::as_select())
                .load(conn)
        })
        .await
        .map_err(log_then_replace_err!(
            "failed to load machines",
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Could not read machines from the database".to_owned(),
            )
        ))?;

    // Generate a random context id
    let context_id: ContextIdBytes = ContextId::new_v4().into();
    let machine_reservations: Vec<MachineReservation>= reservation_logic::resolve_rsd(machines, &rsd, context_id).map_err(|e| {
        match e {
            RsdResolutionError::NoFreeCompatibleMachine { machine_id} => (StatusCode::UNPROCESSABLE_ENTITY, format!("Currently, there is no free machine satisfying the constraints for machine: {}. Please try again later", machine_id)),
            RsdResolutionError::NoCompatibleMachine { machine_id} => (StatusCode::UNPROCESSABLE_ENTITY, format!("There is no machine satisfying the constraints for machine: {}. Please adjust your constraints", machine_id)) 
        }
    })?;
    // Before we update the database we secure a guaranteed spot in the context manager's message queue
    let Ok(context_manager_tx_permit) = dependencies.ctx_manager_tx.reserve().await
        .inspect_err(log_err!("unable to get permit to send a message to the context manager. Maybe the context manager has been shutdown?")) else {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, String::from("Could not prepare context for further use")));
    };

    // Now we want to block the thread while we insert the context into the database.
    let machine_reservation_ids: Vec<db_interaction::models::aliases::MachineId> =
        machine_reservations
            .iter()
            .map(|reservation| reservation.id)
            .collect();

    let context_lifetime = Duration::from(dependencies.config.context_lifetime);
    let context_creation_time = Utc::now();
    let context_timeout = context_creation_time + context_lifetime;

    dependencies
        .db_facade
        .execute_on_current_thread(move |conn| {
            conn.immediate_transaction::<(), diesel::result::Error, _>(|conn| {
                let span = error_span!("context_insertion_transaction");
                let _entered = span.enter();
                // Insert the context id
                ContextIdentifier { id: context_id }
                    .insert_into(contexts_schema::table)
                    .execute(conn)
                    .inspect_err(|_| error!("context id insertion failed"))?;
                // Insert the context's lifetime
                ContextLifetime {
                    context_id,
                    created: context_creation_time.naive_utc(),
                    timeout: context_timeout.naive_utc(),
                }
                .insert_into(context_lifetimes_schema::table)
                .execute(conn)
                .inspect_err(|_| error!("context lifetime insertion failed"))?;
                // Check that the machines are still free. As the Context API queues up reservations this check can only fail
                // if another process (for instance the Maintainer CLI) has interferred with the machines while
                // we were running the reservation logic.
                let taken_machine: Option<MachineId> = machines_schema::table
                    .select(machines_schema::columns::id)
                    .filter(machines_schema::columns::id.eq_any(&machine_reservation_ids))
                    .filter(machines_schema::columns::state.ne(ModelMachineState::Free))
                    .first(conn)
                    .optional()?;
               if let Some(taken_machine_id) = taken_machine {
                  error!(%taken_machine_id, "machine to be reserved for context has been taken by another process before context reservation completed. Aborting context reservation"); 
                  return Err(diesel::result::Error::RollbackTransaction);
               }

                // Mark the machines as reserved
                diesel::update(machines_schema::table)
                    .set(machines_schema::state.eq(ModelMachineState::Reserved))
                    .filter(machines_schema::id.eq_any(&machine_reservation_ids))
                    .execute(conn)
                    .inspect_err(|_| error!("marking machines as reserved failed"))?;
                // Finally update the machine reservations table to declare that the machines are assigned to the new context
                machine_reservations
                    .insert_into(machine_reservations_schema::table)
                    .execute(conn)
                    .inspect_err(|_| error!("inserting machine reservations failed"))?;
                Ok(())
            })
        })
        .await
        .map_err(log_then_replace_err!(
            "writing the context to the database failed",
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                String::from("Could not write the context to the database"),
            )
        ))?;

    // Inform the context manager about the new context
    context_manager_tx_permit.send(ContextManagerMessage::New {
        ctx_id: context_id,
        timeout: context_lifetime,
    });
    Ok(context_id.to_string())
}
