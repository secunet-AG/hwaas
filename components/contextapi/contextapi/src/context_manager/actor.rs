// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use chrono::Utc;
use db_interaction::{
    connection::DbFacade,
    models::{
        context_id::ContextIdBytes,
        contexts::{ContextIdentifier, ContextLifetime},
        machines::MachineState,
    },
    schema::{
        context_lifetimes as context_lifetimes_schema, contexts, machine_reservations,
        machines as machines_schema,
    },
};
use diesel::prelude::*;
use error_stack::{Report, ResultExt};
use error_utils::log_err;
use futures::StreamExt;
use image_api::ImageHandler;

use tokio::{
    sync::mpsc::{Receiver, Sender},
    task::{JoinHandle, JoinSet},
};
use tokio_stream::StreamMap;
use tracing::{debug, error, info, info_span, instrument, trace, warn};

use crate::{
    NetCtrlClient,
    context_manager::{context_permit::ContextAccess, context_termination::terminate_context},
    remote_client::RemoteClient,
};

use super::message::ContextManagerMessage;

/// The number of messages that can be in the queue to the actor before
/// senders need to wait.
const MESSAGE_QUEUE_SIZE: usize = 64;

/// The duration before we let a context deletion task time out, in which
/// case the actor will take care of restarting it from the beginning.
const CONTEXT_TERMINATION_TIMEOUT: Duration = Duration::from_secs(20 * 60);

/// The return type of [`spawn_context_manager`]. This struct contains
/// items that may be used to communciate with the context manager task.
pub(crate) struct ContextManagerCommunication {
    /// For communcating with the context manager.
    pub(crate) context_manager_tx: Sender<ContextManagerMessage>,
    /// Can be used to abort the context manager. This should only be done
    /// when shutting down the context api.
    pub(crate) join_handle: JoinHandle<()>,
}

/// Attempts to spawn a context manager that runs in the background until it is aborted.
///
/// If the launch is successful [`ContextManagerCommunication`] is returned which should
/// be utilized to communicate with the context manager from elsewhere in the
/// application.
///
/// # Side effects
///
/// Apart from launching a long running background task this function has the following
/// additional side effects.
///
/// - All contexts with machines marked as `Resetting` in the database will be terminated
///   (thus eventually making the machines available once more).
///
/// - Each context in the database (that is not being reset) will spawn tasks that sleep
///   until they are expected to timeout and notify the context manager that it should
///   delete the context. Note that these timeout tasks can be replaced by sending a
///   [`ContextManagerMessage::Timeout`] to the context manager.
pub(crate) async fn spawn_context_manager(
    db_facade: Arc<DbFacade>,
    net_ctrl_client: NetCtrlClient,
    remote_client: RemoteClient,
    image_handler: ImageHandler,
) -> Result<ContextManagerCommunication, error_stack::Report<ContextManagerStartupError>> {
    let startup_output = prepare_context_manager_launch(
        db_facade.clone(),
        net_ctrl_client.clone(),
        remote_client.clone(),
        image_handler.clone(),
    )
    .await?;
    let sender = startup_output.sender.clone();
    let join_handle = tokio::spawn(run_context_manager(
        db_facade,
        net_ctrl_client,
        remote_client,
        startup_output,
        image_handler,
    ));

    Ok(ContextManagerCommunication {
        context_manager_tx: sender,
        join_handle,
    })
}

/// This runs the context manager actor.
///
/// It is responsible for:
///
/// - Handing out exclusive access to use contexts to handlers upon their request.
/// - Managing context timeouts.
/// - Deleting contexts upon the request of a handler, or when they otherwise time out.
#[instrument(skip_all)]
async fn run_context_manager(
    db_facade: Arc<DbFacade>,
    net_ctrl_client: NetCtrlClient,
    remote_client: RemoteClient,
    preparation_output: ContextManagerPreparationOutput,
    image_handler: ImageHandler,
) -> () {
    let ContextManagerPreparationOutput {
        mut receiver,
        mut termination_tasks,
        active_contexts,
        ..
    } = preparation_output;

    // For granting exclusive access to use a context
    let mut context_access_map = HashMap::new();
    // A stream of access tokens per context.
    let mut context_access_token_stream = StreamMap::new();
    // A stream of a single timeout per context. The map allows us to update the timeouts on the fly.
    let mut context_timeout_stream = StreamMap::new();
    let make_access_token_stream = || {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let rx = Box::pin(async_stream::stream! {
            while let Some(permit_fut) = rx.recv().await {
                let (return_with, permit)  = permit_fut.await;
                yield (return_with, permit);

            }
        });
        (tx, rx)
    };

    // NOTE: `make_timeout_stream` is not 'static. If this becomes problematic then just clone
    // the sender and db before creating the closure and move them into the closure.
    let make_timeout_stream = |context_id: ContextIdBytes, duration: Duration| {
        let sender = preparation_output.sender.clone();
        let db = db_facade.clone();
        Box::pin(futures::stream::once(async move {
            let span = info_span!("context_timeout_stream", context_id = %context_id);
            // Sleep the given duration
            span.in_scope(|| trace!(seconds = duration.as_secs(), "going to sleep"));
            tokio::time::sleep(duration).await;
            // Confirm with the database that the context really has timed out
            match db
                .spawn_call(move |conn| {
                    context_lifetimes_schema::table
                        .select(ContextLifetime::as_select())
                        .filter(context_lifetimes_schema::context_id.eq(context_id))
                        .first(conn)
                        .optional()
                })
                .await
            {
                Ok(Some(context_lifetime)) => {
                    // Check that the timeout in the database is in the past
                    let context_lifetime_timeout = context_lifetime.timeout.and_utc();
                    let now = Utc::now();
                    if context_lifetime_timeout <= now {
                        // This means the context has timed out and should be deleted.
                        span.in_scope(|| {
                            info!("context has timed out. Requesting context termination")
                        });
                        let _ = sender.send(ContextManagerMessage::Delete(context_id)).await;
                    } else {
                        // Compute how long there is left of the context lifetime.
                        // Note that tokio's timers are not 100% accurate. If the
                        // difference is less than a second we will still issue a delete
                        let diff = context_lifetime_timeout - now;
                        let timeout = Duration::from_secs(diff.num_seconds() as u64);
                        if timeout.as_secs() < 1 {
                            span.in_scope(|| {
                                info!("context is about to timeout. Requesting context termination")
                            });
                            let _ = sender.send(ContextManagerMessage::Delete(context_id)).await;
                        } else {
                            span.in_scope(|| {
                                warn!(
                                    remaining_seconds = timeout.as_secs(),
                                    "timeout extracted from the database is not in the past. Restarting countdown"
                                )
                            });
                            let _ = sender
                                .send(ContextManagerMessage::Timeout {
                                    ctx_id: context_id,
                                    timeout,
                                })
                                .await;
                        }
                    }
                }
                Ok(None) => {
                    // We don't expect this to be the case. The context is probably already deleted, but we issue a deletion command nevertheless.
                    span.in_scope(|| warn!("could not find context lifetime entry for context. Requesting context termination"));
                    let _ = sender.send(ContextManagerMessage::Delete(context_id)).await;
                }
                Err(e) => {
                    span.in_scope(|| {
                        log_err!(
                            "failed to retrieve context lifetime from the database. Retrying later."
                        )(&e)
                    });
                    // We retry by issuing a new timeout command
                    const TIMEOUT_RETRY_DURATION: Duration = Duration::from_secs(5);
                    let _ = sender
                        .send(ContextManagerMessage::Timeout {
                            ctx_id: context_id,
                            timeout: TIMEOUT_RETRY_DURATION,
                        })
                        .await;
                }
            }
        }))
    };

    // Populate the context access map so we can fill the access token streams.
    for (ctx_id, timeout) in active_contexts {
        let (tx, rx) = make_access_token_stream();
        context_access_token_stream.insert(ctx_id, rx);
        context_access_map.insert(ctx_id, (ContextAccess::new(), tx));
        // Also populate the stream map of timeouts.
        context_timeout_stream.insert(ctx_id, make_timeout_stream(ctx_id, timeout));
    }

    // This is the main actor loop where we handle incoming messages
    loop {
        tokio::select! {
            Some(message) = receiver.recv() => {
                match message {
                     ContextManagerMessage::New{ctx_id, timeout } => {
                        // We have a new context to manage which handlers will need to
                        // obtain access tokens for.
                        let (tx, rx) = make_access_token_stream();
                        context_access_token_stream.insert(ctx_id, rx);
                        context_access_map.insert(ctx_id, (ContextAccess::new(), tx));
                        // We also start a timeout for the context
                        context_timeout_stream.insert(ctx_id, make_timeout_stream(ctx_id, timeout));
                     },
                     ContextManagerMessage::GetPermit{ctx_id, return_with} => {
                         // This means that a handler has expressed a desire to get exclusive access to
                         // the given context. We forward the request to the context's access token stream
                         // together with the one shot sender for contacting the handler once the access
                         // token becomes available.
                         let Some((context_access, tx)) = context_access_map.get(&ctx_id) else {
                             let _ = return_with.send(None);
                             continue;
                         };
                         let context_access_token_fut = context_access.acquire(ctx_id);
                         let _ = tx.send(async move {(return_with, context_access_token_fut.await)}).inspect_err(|_| error!(context_id = %ctx_id, "failed to stream context permit"));
                     }
                     ContextManagerMessage::Delete(ctx_id) => {
                        // Remove the stream of access tokens for this context.
                        // This means that all handler's waiting for exclusive use of this context
                        // will necessarily time out.
                        let stream = context_access_token_stream.remove(&ctx_id);
                        drop(stream);
                        // Also remove the pending timeout for the context
                        let stream = context_timeout_stream.remove(&ctx_id);
                        drop(stream);
                        // We remove the access granter as it is no longer needed.
                        if let Some((context_access, _)) = context_access_map.remove(&ctx_id) {
                            let db_facade = db_facade.clone();
                            let net_ctrl_client = net_ctrl_client.clone();
                            let remote_client = remote_client.clone();
                            let image_handler = image_handler.clone();
                            termination_tasks.spawn(async move {
                                // Wait for the previously handed out access token to be dropped to avoid
                                // race conditions one resources associated with the context.
                                context_access.acquire(ctx_id).await;
                                terminate_context(
                                    ContextIdentifier { id: ctx_id },
                                    db_facade,
                                    net_ctrl_client,
                                    remote_client,
                                    CONTEXT_TERMINATION_TIMEOUT,
                                    image_handler,
                                )
                                .await
                            });
                        } else {
                            warn!(context_id = %ctx_id, "context not found. Maybe it is already (being) deleted?");
                        }
                     },
                     ContextManagerMessage::Timeout{ctx_id, timeout} => {
                         // Update the timeout.
                         dbg!(timeout);
                         if let Some(old_timeout) = context_timeout_stream.insert(ctx_id, make_timeout_stream(ctx_id, timeout)) {
                             trace!(seconds = timeout.as_secs(), context_id = %ctx_id, "replaced timeout for context");
                             drop(old_timeout);
                         } else {
                             trace!(seconds = timeout.as_secs(), context_id = %ctx_id, "started timeout for context");
                         }
                     }
                };
            },
            Some((ctx_id,  (return_with, access_token))) = context_access_token_stream.next(), if !context_access_token_stream.is_empty() => {
                // This means that a requested access token for `ctx_id` has become available. We forward it
                // to the handler that asked for it.
                trace!(context_id = %ctx_id, "handing out context access token");
                 let _ = return_with.send(Some(access_token)).inspect_err(|_| debug!(context_id = %ctx_id, "failed to send context permit"));
            },
            context_termination_result = termination_tasks.join_next(), if !termination_tasks.is_empty() => {
                match context_termination_result {
                    Some(Ok(Err(ctx_id))) => {
                        // This means the context termination task timed out before it succeeded
                        // we respawn the task
                            let db_facade = db_facade.clone();
                            let net_ctrl_client = net_ctrl_client.clone();
                let remote_client = remote_client.clone();
                            let image_handler = image_handler.clone();
                            termination_tasks.spawn(async move {
                                terminate_context(
                                    ctx_id,
                                    db_facade,
                                    net_ctrl_client,
                                    remote_client,
                                    CONTEXT_TERMINATION_TIMEOUT,
                                    image_handler,
                                )
                                .await
                            });
                    },
                    Some(Err(e)) => {
                        // This should hopefully never occur. In my understanding it can only happen if the spawned task panics which we should have avoided.
                        error!(error.dbg = ?e, error.msg = %e, "unexpected join error for context termination task. Resources may never get freed. Consider restarting the Context API");
                    },
                    _ => {
                        // This means a context was successfully terminated. The task itself will log the event so there is nothing to do here.
                    }

                }
            },
            Some((ctx_id, _)) = context_timeout_stream.next(), if !context_timeout_stream.is_empty() => {
                // We don't really need to do anything once futures complete here as they will have already
                // sent a message which will be handled in the "Delete" branch above.
                trace!(context_id = %ctx_id, "context timed out");
            }
        };
    }
}

/// This struct contains outputs from the launched tasks when
/// preparing the context manager.
struct ContextManagerPreparationOutput {
    /// This will be used for communicating with the context manager.
    sender: Sender<ContextManagerMessage>,
    /// This will be used by the context manager to receive messages.
    receiver: Receiver<ContextManagerMessage>,
    /// This is a set of spawned context termination tasks.
    termination_tasks: JoinSet<Result<(), ContextIdentifier>>,
    /// This is the contexts considered to be active at the moment
    /// the context manager gets launched. The durations represent
    /// the remaining lifetime of each context.
    active_contexts: HashMap<ContextIdBytes, Duration>,
}

/// Error we return if we fail to startup the context manager.
///
/// This is intended to be wrapped in error_stack::Report.
#[derive(Debug)]
pub struct ContextManagerStartupError;

impl std::fmt::Display for ContextManagerStartupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("could not start context manager")
    }
}

impl std::error::Error for ContextManagerStartupError {}

/// This prepares the context manager and has the following side effects.
///
/// - All contexts with machines marked as resetting in the database
///   as well as those that have timed out will be terminated. The
///   returned output contains a joinset of all of these spawned tasks.
async fn prepare_context_manager_launch(
    db_facade: Arc<DbFacade>,
    net_ctrl_client: NetCtrlClient,
    remote_client: RemoteClient,
    image_handler: ImageHandler,
) -> Result<ContextManagerPreparationOutput, Report<ContextManagerStartupError>> {
    let (sender, receiver) =
        tokio::sync::mpsc::channel::<ContextManagerMessage>(MESSAGE_QUEUE_SIZE);

    // Read all context ids
    let context_ids: Vec<ContextIdentifier> = db_facade
        .execute_on_current_thread(|conn| {
            contexts::table
                .select(ContextIdentifier::as_select())
                .load(conn)
        })
        .await
        .change_context(ContextManagerStartupError)
        .attach_printable("could not load context ids")?;

    // We will log this number later.
    let num_contexts = context_ids.len();

    // Find the contexts that were in the middle of a reset when the Context API was last turned off.
    let resetting_contexts: Vec<ContextIdentifier> = db_facade
        .execute_on_current_thread(|conn| {
            machine_reservations::table
                .inner_join(db_interaction::schema::machines::table)
                .inner_join(db_interaction::schema::contexts::table)
                .select(ContextIdentifier::as_select())
                .filter(machines_schema::state.ne(MachineState::Reserved))
                .distinct()
                .load::<ContextIdentifier>(conn)
        })
        .await
        .change_context(ContextManagerStartupError)
        .attach_printable("unable to load ids of contexts that should be reset")?;

    let mut resetting_contexts: HashSet<ContextIdBytes> =
        resetting_contexts.into_iter().map(Into::into).collect();

    // Also load context lifetimes
    let context_lifetimes: Vec<ContextLifetime> = db_facade
        .execute_on_current_thread(|conn| {
            context_lifetimes_schema::table
                .select(ContextLifetime::as_select())
                .load(conn)
        })
        .await
        .change_context(ContextManagerStartupError)
        .attach_printable("unable to load context lifetimes")?;

    let mut active_contexts = HashMap::new();

    for context_lifetime in context_lifetimes {
        let ctx_id = context_lifetime.context_id;
        let timeout = context_lifetime.timeout.and_utc();
        if context_lifetime.timeout.and_utc() <= Utc::now() {
            // This means the context has timed out and should be reset.
            resetting_contexts.insert(ctx_id);
        } else if !resetting_contexts.contains(&ctx_id) {
            // This means that the context is active. We compute the duration until it
            // times out:
            let diff = timeout - Utc::now();
            let timeout = Duration::from_secs(diff.num_seconds() as u64);
            active_contexts.insert(ctx_id, timeout);
        }
    }

    trace!(
        ?resetting_contexts,
        "reset tasks will be spawned for these contexts during startup"
    );

    let num_resetting_contexts = resetting_contexts.len();

    let num_active_contexts = active_contexts.len();

    // Ensure that there are no active contexts without lifetimes. This is currently not
    // permitted (i.e. we expect the contexts and context_lifetimes tables to have the same size).
    let mut active_context_identifiers = context_ids;
    active_context_identifiers.retain(|ctx_id| !resetting_contexts.contains(&ctx_id.id));
    for ctx_id in active_context_identifiers {
        if !active_contexts.contains_key(&ctx_id.into()) {
            error!(context_id = %ctx_id.id, "BUG: context without lifetime");
            // We don't make this a hard error. The context will just not be available to the application.
        }
    }

    // Spawn all context reset tasks
    let mut termination_tasks = JoinSet::new();

    for context_id in resetting_contexts {
        termination_tasks.spawn(terminate_context(
            context_id.into(),
            db_facade.clone(),
            net_ctrl_client.clone(),
            remote_client.clone(),
            CONTEXT_TERMINATION_TIMEOUT,
            image_handler.clone(),
        ));
    }

    info!(
        num_contexts,
        num_active_contexts, num_resetting_contexts, "startup metrics"
    );

    Ok(ContextManagerPreparationOutput {
        active_contexts,
        sender,
        receiver,
        termination_tasks,
    })
}
