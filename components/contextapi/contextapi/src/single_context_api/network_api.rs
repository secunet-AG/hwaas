// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::HashMap,
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use crate::{ContextApiConfig, NetCtrlClient, path_params::PathParamsNetwork};
use aide::{
    axum::{ApiRouter, IntoApiResponse, routing::get_with},
    transform::{TransformOperation, TransformPathItem},
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use axum::{
    extract::{FromRef, FromRequestParts, WebSocketUpgrade},
    http::Uri,
};
use context_data_structures::aliases::MachineNetworkInterface;
use context_data_structures::aliases::{MachineName, NetworkNameStr};
use context_data_structures::network::AddOp;
use context_data_structures::network::NetworkSetupPatchOp;
use context_data_structures::network::RemoveOp;
use context_data_structures::network::TaggedMachineNetworkInterface;
use context_data_structures::{
    aliases::NetworkName,
    network::{NetworkSetup, NetworkSetupPatch},
};
use db_interaction::models::machines::EnabledPort;
use db_interaction::models::machines::SwitchConnection;
use db_interaction::models::machines::SwitchPort;
use db_interaction::models::networks::Network;
use db_interaction::models::networks::NetworkIdentifier;
use db_interaction::models::{aliases::NetworkId, contexts::ContextIdentifier};
use db_interaction::schema;
use db_interaction::{connection::DbFacade, models::context_id::ContextIdBytes};
use diesel::prelude::*;
use error_utils::{log_err, log_then_replace_err};
use std::collections::HashSet;
use std::ops::ControlFlow;
use tokio::sync::Notify;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::error;
use tracing::instrument;
use tracing::trace;
use tracing::warn;
use tracing::{debug, info_span};
use tracing::{error_span, info};

use super::{
    GuardedContext,
    websocket::{connect_websockets, create_websocket},
};

use crate::context_manager::ContextAccessToken;
pub(crate) type WebSocketCancellationSignals = HashMap<NetworkId, CancellationToken>;

#[derive(Clone)]
pub(crate) struct NetworkApiState {
    db_facade: Arc<DbFacade>,
    pub(crate) ws_cancellation_signals: Arc<Mutex<WebSocketCancellationSignals>>,
    net_ctrl: NetCtrlClient,
    config: Arc<ContextApiConfig>,
}

impl NetworkApiState {
    pub fn new(
        db_facade: Arc<DbFacade>,
        net_ctrl: NetCtrlClient,
        config: Arc<ContextApiConfig>,
    ) -> Self {
        Self {
            db_facade,
            net_ctrl,
            config,
            ws_cancellation_signals: Default::default(),
        }
    }
}

pub(crate) fn network_api_router<S>() -> ApiRouter<S>
where
    S: Send + Sync + Clone + 'static,
    GuardedContext: FromRequestParts<S>,
    NetworkApiState: FromRef<S>,
{
    ApiRouter::new()
        .api_route_with(
            "/:network/websocket",
            get_with(handle_connect_network_request, api_method_doc_ws),
            api_doc_network_api,
        )
        .api_route_with(
            "/:network",
            get_with(handle_get_network_setup, api_method_doc_get)
                .delete_with(handle_delete_network_setup, api_method_doc_delete)
                .put_with(handle_create_network_setup, api_method_doc_put)
                .patch_with(handle_patch_network_setup, api_method_doc_patch),
            api_doc_network_api,
        )
        .api_route_with(
            "/",
            get_with(get_all_networks, api_method_doc_get_all_networks),
            api_doc_network_api,
        )
}

fn api_doc_network_api(op: TransformPathItem) -> TransformPathItem {
    op.tag("Network API")
}

fn api_method_doc_ws(op: TransformOperation) -> TransformOperation {
    op.description(
        "Establish a websocket connection to the network. \
                    The websocket transports L2 traffic (ethernet frames). \
                    Sending a message injects packets. \
                    Receiving messages equals to receiving L2 Network Packets",
    )
    .summary("Connect to network via websocket")
}

fn api_method_doc_get(op: TransformOperation) -> TransformOperation {
    op.description(
        "Read the NetworkSetup associated with the given network. \
      The returned network setup is a cached view that will be accurate \
      most of the time (assuming hardware failure and drop outs to be rare)",
    )
    .summary("Get NetworkSetup")
    .response::<200, Json<NetworkSetup>>()
    .response_with::<408, String, _>(|op| op.description("Request took too long"))
    .response_with::<404, String, _>(|op| {
        op.description("The network was not registered or has been deleted")
    })
}

fn api_method_doc_delete(op: TransformOperation) -> TransformOperation {
    op.description("Disable all ports that are connected to the network")
        .summary("Delete a NetworkSetup")
        .response::<200, ()>()
        .response_with::<408, String, _>(|op| op.description("Request took too long"))
        .response_with::<500, String, _>(|op| op.description("Could not disable one or more ports"))
}

fn api_method_doc_put(op: TransformOperation) -> TransformOperation {
    op.description("Create (or update) a network that connects the interfaces in the NetworkSetup")
        .summary("Put NetworkSetup")
        .response::<200, ()>()
        .response_with::<500, String, _>(|op| {
            op.description("Failure due to port enabling, disabling and/or interface reservation")
        })
        .response_with::<422, String, _>(|op| {
            op.description("Network interface not known to the context")
        })
        .response_with::<408, String, _>(|op| op.description("Request took too long"))
}

fn api_method_doc_patch(op: TransformOperation) -> TransformOperation {
    op.description("Update a network by connecting and/or disconnecting interfaces in accordance with the given json-patch")
    .summary("Patch NetworkSetup")
    .response::<200, ()>()
    .response_with::<500, String, _>(|op| {
        op.description("Failure due to port enabling, disabling and/or interface reservation")
    })
    .response_with::<409, String, _>(|op| {
        op.description("Failure due to an invalid patch operation")
    })
    .response_with::<408, String, _>(|op| op.description("Request took too long"))
    .response_with::<422, String, _>(|op| op.description("Network interface not known to the context"))
}

fn api_method_doc_get_all_networks(op: TransformOperation) -> TransformOperation {
    op.description("Get the list of names for all allocated networks")
        .summary("List all network names")
        .response_with::<200, Json<Vec<NetworkName>>, _>(|op| {
            op.description("List of all network names")
        })
}

/// This struct wrapps an argument and async closure pair
/// that gets called in a spawned task upon drop. This is useful
/// to ensure crucial database writes occur when handlers timeout.
struct OnDropSpawnCallback<A, T, F, Fut>
where
    A: Send + 'static,
    T: Send + 'static,
    Fut: std::future::Future<Output = T> + Send + 'static,
    F: FnOnce(A) -> Fut,
{
    state_with_function: Option<(A, F)>,
    _output: PhantomData<Fut>,
    _output_type: PhantomData<T>,
}

impl<A, T, F, Fut> Drop for OnDropSpawnCallback<A, T, F, Fut>
where
    A: Send + 'static,
    T: Send + 'static,
    Fut: std::future::Future<Output = T> + Send + 'static,
    F: FnOnce(A) -> Fut,
{
    fn drop(&mut self) {
        if let Some((state, f)) = self.state_with_function.take() {
            // Spawn the callback. We immediately drop the join handle as we can't do anything with it
            // (recall that this does not terminate the spawned task).
            std::mem::drop(tokio::spawn(f(state)));
        }
    }
}

impl<A, T, F, Fut> OnDropSpawnCallback<A, T, F, Fut>
where
    A: Send + 'static,
    T: Send + 'static,
    Fut: std::future::Future<Output = T> + Send + 'static,
    F: FnOnce(A) -> Fut,
{
    fn new(arg: A, callback: F) -> Self {
        Self {
            state_with_function: Some((arg, callback)),
            _output: PhantomData,
            _output_type: PhantomData,
        }
    }
}
/// Connects all the interfaces declared in
/// `network_setup` to the `network`.
///
/// If `network` has previously been established
/// then all interfaces that are currently connected to it,
/// but are not in `network_setup` will be disconnected.
///
/// # Errors
///
/// Connecting (resp. disconnecting) interfaces
/// to (resp. from) a network is a fallible operation.
///
/// This handler tries to connect and disconnect as
/// many interfaces as possible and accumulates all
/// errors rather than immediately giving up.
#[allow(clippy::let_with_type_underscore)]
#[tracing::instrument(skip(dependencies))]
async fn handle_create_network_setup(
    GuardedContext(context_access_token): GuardedContext,
    State(dependencies): State<NetworkApiState>,
    Path(PathParamsNetwork { network }): Path<PathParamsNetwork>,
    Json(network_setup): Json<NetworkSetup>,
) -> impl IntoApiResponse {
    let context_id = context_access_token.context_id;
    let ports_to_be_connected =
        network_setup_to_switch_port_data(context_id, &network_setup, &dependencies.db_facade)
            .await?;
    upsert_network(
        ports_to_be_connected,
        network,
        context_access_token,
        dependencies,
    )
    .await
}

/// Disconnects all interfaces from the `network`.
///
///
/// # Errors
///
/// Disconnecting an interface is a fallible operation
/// and we try to disconnect as many interfaces
/// as possible without terminating on the first observed error.
#[allow(clippy::let_with_type_underscore)]
#[tracing::instrument(skip(dependencies))]
async fn handle_delete_network_setup(
    GuardedContext(context_access_token): GuardedContext,
    State(dependencies): State<NetworkApiState>,
    Path(PathParamsNetwork { network }): Path<PathParamsNetwork>,
) -> impl IntoApiResponse {
    let ws_delete_signals = dependencies.ws_cancellation_signals.clone();

    // Load enabled ports belonging to the network.
    let ctx_id = context_access_token.context_id;
    let load_error = (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Could not load network data from the database",
    );
    let maybe_network_with_switch_ports: Option<(Network, Vec<SwitchPort>)> = {
        dependencies.db_facade.spawn_call(move |conn| {
            let network: Option<Network> = schema::networks::table.select(Network::as_select()).filter(
                 schema::networks::context_id.eq(ctx_id).and(schema::networks::name.eq(network))
                    ).get_result(conn).optional().inspect_err(|e| {
                        error!(error.dbg = ?e, error.msg = %e, "failed to load network");
                    })?;
            let Some(network) = network else {
                // For some reason type annotations are needed here
                return Result::<_, diesel::result::Error>::Ok(None);
                    };
            let ports_to_disconnect = EnabledPort::belonging_to(&network).inner_join(schema::switch_ports::table).select(SwitchPort::as_select()).load(conn)?;
                    Ok(Some((network, ports_to_disconnect)))
                }).await.map_err(|e| {
                    error!(error.dbg = ?e, error.msg = %e, "failed to load enabled ports belonging to the network");
                    load_error
                })
    }?;

    let Some((network, ports_to_disconnect)) = maybe_network_with_switch_ports else {
        return Err((
            StatusCode::NOT_FOUND,
            "The network was not found. Maybe it has already been deleted?",
        ));
    };

    // Notify all connected websockets that the network is going to be deleted.
    {
        if let Ok(mut lock_guard) = ws_delete_signals
            .lock()
            .inspect_err(log_err!("unexpected poison error"))
        {
            if let Some(delete_signal) = lock_guard.remove(&network.id) {
                delete_signal.cancel();
            } else {
                warn!("no websocket cancellation token found for the network");
            }
        }
    }

    debug!(
        ?ports_to_disconnect,
        "going to disconnect all switch ports assigned to the network"
    );
    let num_ports_to_disconnect = ports_to_disconnect.len();

    let net_ctrl = dependencies.net_ctrl.clone();
    let network_id = network.id;

    // Create a callback that handles updating the enabled ports table in the database.
    // We will place this in a special drop guard to ensure that it gets called even if the
    // handler times out. We use channel to get the outcome in the case where we drop this
    // ourselves to let the update run.
    let (tx, db_update_recv) = tokio::sync::oneshot::channel::<bool>();

    let update_switch_ports_in_db = |disconnected_ports: Vec<_>| async move {
        // Move the context access token so the context may not be accessed before the update has completed.
        let context_access_token = context_access_token;
        let num_disconnected_ports = disconnected_ports.len();

        let all_switch_ports_disconnected = num_disconnected_ports == num_ports_to_disconnect;
        let update_result = dependencies
            .db_facade
            .spawn_writing_call(move |conn| {
                let context_id = context_access_token.context_id;
                let span = error_span!("delete_network_from_db", network_id, %context_id);
                let _entered = span.enter();
                if !disconnected_ports.is_empty() {
                    diesel::delete(schema::enabled_ports::table)
                        .filter(schema::enabled_ports::id.eq_any(&disconnected_ports))
                        .execute(conn)
                        .inspect_err(|_| error!("failed to delete enabled switch ports"))?;
                }

                if all_switch_ports_disconnected {
                    // This means that the network delete was successful.
                    // Conclude by deleting the network from the database
                    diesel::delete(schema::networks::table)
                        .filter(schema::networks::id.eq(network_id))
                        .execute(conn)
                        .inspect_err(|_| error!("failed to delete network"))?;
                }
                Ok(())
            })
            .await
            .inspect_err(log_err!(
                "could not write switch port updates to the database"
            ));
        let _ = tx.send(update_result.is_ok());
    };

    let disconnected_ports = Vec::with_capacity(num_ports_to_disconnect);
    let mut pending_db_write =
        OnDropSpawnCallback::new(disconnected_ports, update_switch_ports_in_db);

    // Now proceed with actually disconnecting the ports.
    let disconnected_ports = &mut pending_db_write
        .state_with_function
        .as_mut()
        .expect("The field should be set in the on drop spawn callback")
        .0;
    let mut disconnect_task_set = JoinSet::new();
    // Note that the disconnect tasks in the joinset stop once the joinset is dropped.
    for switch_port in ports_to_disconnect {
        let net_ctrl = net_ctrl.clone();
        disconnect_task_set.spawn(async move {
                let switch_port_id = switch_port.id;
                net_ctrl.disable_port(&switch_port).await.inspect_err(|e| error!(?switch_port, error.dbg = ?e, error.msg = %e, "failed to disable switch port")).map(|_| switch_port_id)
            });
    }
    // Set this to true if one of the ports fail to disconnect
    let mut switch_port_disconnect_failed = false;
    while let Some(disconnect_result) = disconnect_task_set.join_next().await {
        if let Ok(Ok(switch_port_id)) = disconnect_result
            .inspect_err(|e| error!(error.dbg = ?e, error.msg = %e, "unexpected join error"))
        {
            disconnected_ports.push(switch_port_id);
        } else {
            switch_port_disconnect_failed = true;
        }
    }
    // As all disconnect tasks have completed we trigger the pending database write
    drop(pending_db_write);
    let db_writes_succeeded = db_update_recv.await.inspect_err(|e| error!(error.msg = %e, error.dbg = ?e, "unable to receive database update result. This is unexpected")).unwrap_or(false);
    if switch_port_disconnect_failed {
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to disconnect one or more ports",
        ))
    } else if !db_writes_succeeded {
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Could not delete the network from the database",
        ))
    } else {
        Ok(())
    }
}

/// This handler function returns a List of all allocated networks
/// and their names
async fn get_all_networks(
    GuardedContext(context_access_token): GuardedContext,
    State(dependencies): State<NetworkApiState>,
) -> Result<Json<Vec<NetworkName>>, (StatusCode, &'static str)> {
    let ctx_id = context_access_token.context_id;
    let network_names: Vec<NetworkName> = dependencies
        .db_facade
        .spawn_call(move |conn| {
            db_interaction::models::networks::Network::belonging_to(&ContextIdentifier::from(
                ctx_id,
            ))
            .select(db_interaction::schema::networks::name)
            .load(conn)
        })
        .await
        .map_err(log_then_replace_err!(
            "failed to load network names from the database",
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Could not load network names from the database",
            )
        ))?;

    Ok(Json(network_names))
}

/// Applies the given patch to the network setup
/// corresponding to the given network name.
///
/// # Errors
///
///
/// ## Specification compliance
///
/// Patches that do not satisfy the requirements of the JSON patch
/// specification will not be applied.
///
///
/// ## Fallible connections
///
/// Connecting (resp. disconnecting) interfaces to (resp. from) a network
/// is a fallible operation. In the case of errors the patch may be left
/// only partially applied. Although the `PATCH` method specification states
/// that patches are applied atomically (either the entire patch is applied or nothing),
/// this is something we cannot guarantee in the case of internal server errors.
#[allow(clippy::let_with_type_underscore)]
#[tracing::instrument(skip(dependencies))]
async fn handle_patch_network_setup(
    GuardedContext(context_access_token): GuardedContext,
    State(dependencies): State<NetworkApiState>,
    Path(PathParamsNetwork { network }): Path<PathParamsNetwork>,
    Json(patch): Json<NetworkSetupPatch>,
) -> impl IntoApiResponse {
    let ctx_id = context_access_token.context_id;
    let network_name = network;
    let maybe_network_setup_and_net_id: Option<(NetworkSetup, NetworkId)> = dependencies
        .db_facade
        .spawn_call(move |conn| {
            let network_id: Option<NetworkId> =
                read_network_id(conn, ctx_id, network_name).optional()?;
            let Some(net_id) = network_id else {
                return Ok(None);
            };
            Ok(Some((read_network_setup(conn, net_id)?, net_id)))
        })
        .await
        .map_err(log_then_replace_err!(
            "failed to read network data",
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Could not load network data from the database",
            )
        ))?;

    let Some((network_setup, network_id)) = maybe_network_setup_and_net_id else {
        return Err((
            StatusCode::NOT_FOUND,
            "Could not find the network. Perhaps it has been deleted?",
        ));
    };

    // Load all machine switch port pairs belonging to the context. We need them to determine whether it even makes sense for the
    // patch to be applied.
    let machines_network_interface_pairs: Vec<(MachineName, MachineNetworkInterface)> =
        dependencies
            .db_facade
            .spawn_call(move |connection| {
                schema::machine_reservations::table
                    .inner_join(schema::switch_connections::table.on(
                        schema::machine_reservations::id.eq(schema::switch_connections::machine_id),
                    ))
                    .filter(schema::machine_reservations::context_id.eq(ctx_id))
                    .select((
                        schema::machine_reservations::machine_name,
                        schema::switch_connections::interface,
                    ))
                    .load(connection)
            })
            .await
            .map_err(log_then_replace_err!(
                "failed to load machine switch port pairs from the database",
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error")
            ))?;

    // Check whether the patch is applicable before applying it
    {
        let mut missing_interface_described_in_path = false;

        let machine_interface_inspector = |machine_name: &str,
                                           network_interface: &str|
         -> ControlFlow<()> {
            if machines_network_interface_pairs
                .iter()
                .any(|(name, interface)| (name == machine_name) && (interface == network_interface))
            {
                ControlFlow::Continue(())
            } else {
                missing_interface_described_in_path = true;
                debug!(
                    network_interface,
                    machine_name,
                    "attempt to add network interface which is not available to the context"
                );
                ControlFlow::Break(())
            }
        };
        if let Err(patch_op) = patch.is_applicable(&network_setup, machine_interface_inspector) {
            error!(?patch_op, "invalid patch operation");
            if missing_interface_described_in_path {
                return Err((
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "The patch describes a machine network interface that is not available to the context",
                ));
            } else {
                return Err((StatusCode::CONFLICT, "invalid patch"));
            }
        }
        // This is no longer needed and may take up a bit of memory, so we simply free it now.
        drop(machines_network_interface_pairs);
    }

    // We will now start updating our network against the patch. We need to keep track off what has been added and remove, to see how long
    // we can delay upserting the network setup.
    let mut added_interfaces = HashSet::new();
    let mut removed_interfaces = HashSet::new();
    let mut network_setup = network_setup;
    let realize_network_setup = |setup: NetworkSetup, context_access_token: ContextAccessToken| async {
        let ports_to_be_connected =
            network_setup_to_switch_port_data(ctx_id, &setup, &dependencies.db_facade).await?;
        let context_access_token = update_network(
            ports_to_be_connected,
            network_id,
            context_access_token,
            dependencies.clone(),
        )
        .await?;
        Ok((setup, context_access_token))
    };
    let reset_book_keeping = |first_set: &mut HashSet<_>, second_set: &mut HashSet<_>| {
        first_set.clear();
        second_set.clear()
    };

    let mut context_access_token = context_access_token;
    for op in patch.0 {
        match op {
            NetworkSetupPatchOp::Add(add_op) => match add_op {
                AddOp::Interface(tagged_machine_interface) => {
                    if removed_interfaces.contains(&tagged_machine_interface) {
                        (network_setup, context_access_token) =
                            realize_network_setup(network_setup, context_access_token).await?;
                        reset_book_keeping(&mut added_interfaces, &mut removed_interfaces);
                    }
                    added_interfaces.insert(tagged_machine_interface.clone());
                    network_setup.insert(tagged_machine_interface);
                }
                AddOp::MachineWithInterfaces {
                    machine,
                    interfaces,
                } => {
                    for interface in interfaces.0 {
                        let tagged_machine_interface = TaggedMachineNetworkInterface {
                            machine_name: machine.clone(),
                            interface,
                        };
                        if removed_interfaces.contains(&tagged_machine_interface) {
                            (network_setup, context_access_token) =
                                realize_network_setup(network_setup, context_access_token).await?;
                            reset_book_keeping(&mut added_interfaces, &mut removed_interfaces);
                        }
                        added_interfaces.insert(tagged_machine_interface.clone());
                        network_setup.insert(tagged_machine_interface);
                    }
                }
            },
            NetworkSetupPatchOp::Remove(remove_op) => match remove_op {
                RemoveOp::TaggedInterface(tagged_machine_interface) => {
                    if added_interfaces.contains(&tagged_machine_interface) {
                        (network_setup, context_access_token) =
                            realize_network_setup(network_setup, context_access_token).await?;
                        reset_book_keeping(&mut added_interfaces, &mut removed_interfaces);
                    }
                    network_setup.remove(
                        &tagged_machine_interface.machine_name,
                        &tagged_machine_interface.interface,
                    );
                    removed_interfaces.insert(tagged_machine_interface);
                }
                RemoveOp::Machine(machine_name) => {
                    if added_interfaces
                        .iter()
                        .any(|tagged_interface| tagged_interface.machine_name == machine_name)
                    {
                        (network_setup, context_access_token) =
                            realize_network_setup(network_setup, context_access_token).await?;
                        reset_book_keeping(&mut added_interfaces, &mut removed_interfaces);
                    }
                    let Some(interfaces) = network_setup.0.remove(&machine_name) else {
                        warn!(
                            machine_name,
                            "BUG: no interfaces for machine in the network setup, despite patch being valid"
                        );
                        continue;
                    };
                    removed_interfaces.extend(interfaces.0.into_iter().map(|interface| {
                        TaggedMachineNetworkInterface {
                            machine_name: machine_name.clone(),
                            interface,
                        }
                    }));
                }
            },
        }
    }
    if (!added_interfaces.is_empty()) || (!removed_interfaces.is_empty()) {
        realize_network_setup(network_setup, context_access_token)
            .await
            .map(|_| ())
    } else {
        Ok(())
    }
}

/// Returns the `NetworkSetup` for the
/// given `network` if it exists.
#[allow(clippy::let_with_type_underscore)]
#[tracing::instrument(skip(dependencies))]
async fn handle_get_network_setup(
    GuardedContext(context_access_token): GuardedContext,
    State(dependencies): State<NetworkApiState>,
    Path(PathParamsNetwork { network }): Path<PathParamsNetwork>,
) -> impl IntoApiResponse {
    let context_id = context_access_token.context_id;
    let network_setup: Option<NetworkSetup> = dependencies
        .db_facade
        .spawn_call(move |conn| {
            let network_id: Option<NetworkId> =
                read_network_id(conn, context_id, network).optional()?;

            let Some(net_id) = network_id else {
                return Ok(None);
            };
            Ok(Some(read_network_setup(conn, net_id)?))
        })
        .await
        .map_err(log_then_replace_err!(
            "network setup load failed",
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Could not read the network setup from the database",
            )
        ))?;

    network_setup.map(Json).ok_or((
        StatusCode::NOT_FOUND,
        "The network could not be found. Perhaps it has been deleted?",
    ))
}

/// On a `WebSocketUpgrade`, look up the network and establish
/// a connection to the ws-gateway. Once the connection is established connect
/// both websockets and relay the traffic by calling [`relay_websocket`].
#[allow(clippy::let_with_type_underscore)]
#[tracing::instrument(skip(dependencies))]
async fn handle_connect_network_request(
    GuardedContext(context_access_token): GuardedContext,
    State(dependencies): State<NetworkApiState>,
    Path(PathParamsNetwork { network }): Path<PathParamsNetwork>,
    ws: WebSocketUpgrade,
) -> impl IntoApiResponse {
    // Get the associated network data
    let context_id = context_access_token.context_id;
    let net_id: Option<NetworkId> = dependencies
        .db_facade
        .spawn_call(move |conn| lookup_network_id(conn, context_id, &network).optional())
        .await
        .map_err(log_then_replace_err!(
            "network id lookup failed",
            (StatusCode::INTERNAL_SERVER_ERROR, "Network lookup failed")
        ))?;

    let Some(net_id) = net_id else {
        return Err((
            StatusCode::NOT_FOUND,
            "The network was not found. Perhaps it has been deleted?",
        ));
    };

    let socket_gateway_uri = format!(
        "{}/ws/{}",
        dependencies.config.network_gateway.ws_gateway_url, net_id
    )
    .parse::<Uri>()
    .map_err(|e| {
        error!("Could not build URL: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Unexpected error occurred while handling the request",
        )
    })?;

    let socket_gateway = create_websocket(socket_gateway_uri).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal error occurred while trying to handle the websocket connection",
        )
    })?;

    // Get a cancellation token from the websocket cancellation signals map. If an entry does not already exist for this network we should create one.
    let ws_cancellation_signal = {
        let Ok(mut ws_cancellations_signals_lock) = dependencies
            .ws_cancellation_signals
            .lock()
            .inspect_err(log_err!("unexpected poison error"))
        else {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Something went wrong. Unexpected crash detected: Please contact the HWaaS maintainers",
            ));
        };
        ws_cancellations_signals_lock
            .entry(net_id)
            .or_insert_with(CancellationToken::new)
            .clone()
    };

    // The websocket should be closed if either the network or entire context is deleted.
    let context_termination_signal = context_access_token.cancelled_owned();
    let cancellation_signal = Box::pin(async move {
        let direct_delete = std::pin::pin!(ws_cancellation_signal.cancelled());
        let context_termination = std::pin::pin!(context_termination_signal);
        futures::future::select(direct_delete, context_termination).await;
    });
    Ok::<_, (StatusCode, &'static str)>(ws.on_upgrade(move |socket_user| {
        connect_websockets(socket_user, socket_gateway, cancellation_signal)
    }))
}

/// A switch port that might be enabled. If a network id
/// is present then the port is enabled and connected to
/// the corresponding network.
type MaybeEnabledSwitchPort = (SwitchPort, Option<NetworkId>);

/// Load the switch ports described corresponding to the machine,
/// interface pairs of the network setup.
///
/// For each of the switch ports that are connected to a network
/// we also return the id of the network it is connected to (and
/// `Option::None` if it is not connected).
#[instrument(skip(db_facade))]
async fn network_setup_to_switch_port_data(
    ctx_id: ContextIdBytes,
    network_setup: &NetworkSetup,
    db_facade: &DbFacade,
) -> Result<Vec<MaybeEnabledSwitchPort>, (StatusCode, &'static str)> {
    // Find the ports corresponding to the network setup
    let machine_names: Vec<String> = network_setup.0.keys().map(ToString::to_string).collect();
    let ports_with_machine_and_net_id: Vec<(
        SwitchPort,
        SwitchConnection,
        MachineName,
        Option<NetworkId>,
    )> = db_facade
        .spawn_call(move |conn| {
            schema::switch_ports::table
                .inner_join(schema::switch_connections::table)
                .left_join(
                    schema::enabled_ports::table
                        .on(schema::switch_ports::id.eq(schema::enabled_ports::id)),
                )
                .inner_join(schema::machine_reservations::table.on(
                    schema::switch_connections::machine_id.eq(schema::machine_reservations::id),
                ))
                .select((
                    SwitchPort::as_select(),
                    SwitchConnection::as_select(),
                    schema::machine_reservations::machine_name,
                    schema::enabled_ports::net_id.nullable(),
                ))
                .filter(
                    schema::machine_reservations::context_id
                        .eq(ctx_id)
                        .and(schema::machine_reservations::machine_name.eq_any(machine_names)),
                )
                .load(conn)
        })
        .await
        .map_err(log_then_replace_err!(
            "failed to load swtich port data corresponding to the network setup",
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to read switch port data",
            )
        ))?;
    // Filter out the entries that are not described in the network setup and map the result
    // to the desired type.
    let filtered_ports_with_machine_and_net_id: Vec<MaybeEnabledSwitchPort> =
        ports_with_machine_and_net_id
            .into_iter()
            .filter_map(
                |(switch_port, switch_connection, machine_name, assigned_net_id)| {
                    network_setup
                        .0
                        .get(&machine_name)
                        .map(|network_interfaces| {
                            network_interfaces.0.contains(&switch_connection.interface)
                        })
                        .unwrap_or(false)
                        .then_some((switch_port, assigned_net_id))
                },
            )
            .collect();
    // Check that there are as many switch ports extracted as there are interfaces in the network setup
    let interface_count = network_setup
        .0
        .values()
        .map(|interface_set| interface_set.0.len())
        .sum::<usize>();
    if interface_count == filtered_ports_with_machine_and_net_id.len() {
        Ok(filtered_ports_with_machine_and_net_id)
    } else {
        Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            "the network setup describes one or more machine interface pairs that are not available to the context",
        ))
    }
}

/// Updates the network identified by `network_id`. See [`upsert_network`] for more
/// information.
/// The method needs to hold an owned context access token to ensure exclusive access to the context. The token
/// gets returned to the caller upon success.
#[instrument(skip_all)]
async fn update_network(
    ports_to_be_connected: Vec<MaybeEnabledSwitchPort>,
    network_id: NetworkId,
    context_access_token: ContextAccessToken,
    dependencies: NetworkApiState,
) -> Result<ContextAccessToken, (StatusCode, &'static str)> {
    let db_facade = dependencies.db_facade.clone();
    let net_ctrl = dependencies.net_ctrl;
    // Load all the switch ports that are currently connected to this network
    let mut currently_connected: Vec<SwitchPort> = db_facade
        .spawn_call(move |conn| {
            schema::enabled_ports::table
                .inner_join(schema::switch_ports::table)
                .filter(schema::enabled_ports::net_id.eq(network_id))
                .select(SwitchPort::as_select())
                .load(conn)
        })
        .await
        .map_err(log_then_replace_err!(
            "failed to load switch ports",
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Could not proceed with network update due to a database error",
            )
        ))?;

    // Start tasks to connect and disconnect interfaces.
    let mut disconnect_tasks = JoinSet::new();
    let mut connect_tasks = JoinSet::new();

    let disconnect_port = |switch_port: SwitchPort, net_ctrl: NetCtrlClient| async move {
        debug!(?switch_port, "disabling switch port");
        net_ctrl
            .disable_port(&switch_port)
            .await
            .map_err(log_then_replace_err!("failed to disable switch port", ()))
            .map(|_| switch_port.id)
            .inspect(|_| debug!(?switch_port, "switch port disabled"))
    };

    let connect_port = move |switch_port: SwitchPort, net_ctrl: NetCtrlClient| async move {
        debug!(?switch_port, network_id, "going to connect switch port");
        net_ctrl
                .enable_port(&switch_port, network_id)
                .await
                .inspect_err(
                    |e| error!(error.dbg = ?e, error.msg = %e, ?switch_port, network_id, "failed to enable port")
                )
                .map_err(|_|())
                .map(|_| switch_port.id)
                .inspect(|_| debug!(?switch_port, network_id, "connected switch port"))
    };

    for (switch_port, net_id) in ports_to_be_connected {
        match net_id {
            Some(id) if id == network_id => {
                // In this case we do not want to disconnect the switch port
                if let Some((idx, _)) = currently_connected
                    .iter()
                    .enumerate()
                    .find(|(_, connected_port)| connected_port == &&switch_port)
                {
                    let _ = currently_connected.swap_remove(idx);
                } else {
                    error!(
                        network_id,
                        ?switch_port,
                        "BUG: the switch port should be among the currently connected ports"
                    );
                    debug_assert!(false);
                }
                // We do not bother reconnecting the already connected switch port.
            }
            Some(_) => {
                // This means the port is connected to some other network. We need to first disconnect it
                // then connect it to this network.
                let notify = Arc::new(Notify::new());
                let net_ctrl = net_ctrl.clone();
                let net_ctrl_clone = net_ctrl.clone();
                let notify_clone = notify.clone();
                let switch_port_clone = switch_port.clone();
                disconnect_tasks.spawn(async move {
                    let port_id = disconnect_port(switch_port_clone, net_ctrl_clone).await?;
                    notify.notify_one();
                    Ok(port_id)
                });
                connect_tasks.spawn(async move {
                    notify_clone.notified().await;
                    connect_port(switch_port, net_ctrl).await
                });
            }
            None => {
                // In this case the switch port is currently not connected to any network so we connect
                // it to this one right away
                let net_ctrl = net_ctrl.clone();
                connect_tasks.spawn(async move { connect_port(switch_port, net_ctrl).await });
            }
        }
    }

    // All remaining switch ports in `currently_connected` are different from the given vector of switch ports to
    // connect. Hence they belonged to some previous network setup and should be disconnected
    for switch_port in currently_connected {
        let net_ctrl = net_ctrl.clone();
        disconnect_tasks.spawn(async move { disconnect_port(switch_port, net_ctrl).await });
    }

    // We will loop over the spawned tasks as they complete and record their ids which we then later write to the database. We want the database write
    // to take place even if this future is dropped (due to handler timeout for instance) hence we setup a callback and hand it to an OnDropSpawnCallback.
    // We use channels to obtain the database update result when we get the chance to spawn the update ourselves.
    let (tx, db_update_recv) = tokio::sync::oneshot::channel();
    let db_update_callback = |(connected, disconnected): (Vec<EnabledPort>, Vec<i32>)| async move {
        // note that we move the context access token thus ensuring that the context cannot be extracted until the update has completed.
        let context_access_token_or_error = db_facade.spawn_writing_call(move |conn| {
                // It is important that we handle the disconnects first as we may have disconnected then reconnected a switch port, but
                // never the other way round
                diesel::delete(schema::enabled_ports::table).filter(schema::enabled_ports::id.eq_any(&disconnected)).execute(conn)
                    .inspect_err(|_| error!(disconnected_switch_port_ids = ?disconnected, "failed to delete from the enabled ports table"))?;

                diesel::insert_into(schema::enabled_ports::table).values(&connected).execute(conn)
                    .inspect_err(|_| error!(connected_ports = ?connected,"failed to insert into the enabled ports table"))?;

                Ok(context_access_token)
                }).await;
        let _ = tx.send(context_access_token_or_error);
    };

    let disconnected = Vec::new();
    let connected = Vec::new();
    let mut pending_db_update_guard =
        OnDropSpawnCallback::new((connected, disconnected), db_update_callback);
    let (connected, disconnected) = &mut pending_db_update_guard
        .state_with_function
        .as_mut()
        .expect("The field should be set on on drop spawn callback")
        .0;
    let mut error_occurred = false;
    let mut disconnect_tasks_completed = disconnect_tasks.is_empty();
    let mut connect_tasks_completed = connect_tasks.is_empty();
    loop {
        tokio::select! {
            Some(disconnect_result) = disconnect_tasks.join_next(), if !disconnect_tasks_completed => {
                match disconnect_result.inspect_err(log_err!("unexpected join error")) {
                    Ok(Ok(switch_port_id)) => {disconnected.push(switch_port_id);},
                    _ => {error_occurred = true;},
                }
                disconnect_tasks_completed = disconnect_tasks.is_empty();
            },
            Some(connect_result) = connect_tasks.join_next(), if !connect_tasks_completed => {
                match connect_result.inspect_err(log_err!("unexpected join error")) {
                    Ok(Ok(switch_port_id)) => {connected.push(EnabledPort{id: switch_port_id, net_id: network_id});},
                    _ => {error_occurred = true;}
                }
                connect_tasks_completed = connect_tasks.is_empty();
            },
            else =>  {
                break;
            }
        }
    }

    // Now write the results to the database. Recall that we do this by dropping the pending_db_update_guard.
    drop(pending_db_update_guard);
    let user_facing_error_message =
        "An error occurred when attempting to write a network update to the database";

    let user_facing_error = (StatusCode::INTERNAL_SERVER_ERROR, user_facing_error_message);

    let context_access_token = db_update_recv
        .await
        .map_err(log_then_replace_err!(
            "failed to receive message from spawned database update. This is unexpected behaviour",
            user_facing_error
        ))?
        .map_err(log_then_replace_err!(
            "failed to write network updates to the database",
            user_facing_error
        ))?;

    if error_occurred {
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "The network update was not successful. What we believe to be the current network state can be extracted via the GET method, but note that its accuracy cannot be guaranteed",
        ))
    } else {
        Ok(context_access_token)
    }
}

/// Create or update a network for the context with the given network name.
///
/// The `ports_to_be_connected` should be a list of switch ports together with
/// the id of the network they are currently connected to (if any). In the case
/// where the network id is given, but it differs from the current network we
/// will take care to first disconnect the interface before connecting it to
/// this network.
#[instrument(skip_all)]
async fn upsert_network(
    ports_to_be_connected: Vec<(SwitchPort, Option<NetworkId>)>,
    network_name: NetworkName,
    context_access_token: ContextAccessToken,
    dependencies: NetworkApiState,
) -> Result<(), (StatusCode, &'static str)> {
    let ctx_id = context_access_token.context_id;
    // Lookup or create an identifier for the network. We move the context access token
    // into the spawned database query to ensure that the context may not be accessed
    // until the query has completed.
    let (network_id, context_access_token)  = dependencies.db_facade.spawn_writing_call(move |conn| {
        let span = info_span!("network_id_lookup", ctx_id = %ctx_id, network_name);
        let _entered = span.enter();
        let ctx_id = context_access_token.context_id;
        let network_id = {
            if let Some(network_id) = schema::networks::table
                .select(schema::networks::id)
                .filter(
                    schema::networks::name
                        .eq(&network_name)
                        .and(schema::networks::context_id.eq(ctx_id)),
                )
                .first(conn)
                .optional()
                .inspect_err(|_|error!("network id database search failed"))?
            {
                trace!(network_id, "extracted existing network id");
                Ok(network_id)
            } else {
                trace!("the network does not already exist: attempting to extract an id for it");
                // This means that the network does not already exist hence we need to insert it.
                conn
                    .immediate_transaction::<NetworkId, diesel::result::Error, _>(|conn| {
                        let NetworkIdentifier { id: net_id } = schema::network_identifiers::table
                            .left_join(schema::networks::table)
                            .filter(schema::networks::id.is_null())
                            .select(NetworkIdentifier::as_select())
                            .first(conn)
                            .inspect_err(|_|error!("failed to obtain new network id"))?;
                        Network {
                            id: net_id,
                            name: network_name,
                            context_id: ctx_id,
                        }
                        .insert_into(schema::networks::table)
                        .execute(conn)
                        .inspect_err(|_|error!("could not insert new network into the database"))?;
                        Ok(net_id)
                    })
                    .inspect(|net_id| {
                        info!(
                            network_id = net_id,
                            "inserted new network into the database"
                        )
                    })
                    .inspect_err(|_|error!("network creation transaction failed"))
            }
        }?;
        Ok((network_id, context_access_token))
    }).await.map_err(log_then_replace_err!("could not get network id for the network", (StatusCode::INTERNAL_SERVER_ERROR, "Could not update or insert network. This may be due to a database error, or there may currently be too many active networks")))?;
    update_network(
        ports_to_be_connected,
        network_id,
        context_access_token,
        dependencies,
    )
    .await
    .map(|_| ())
}

/// Look up the id of the network with the given `network_name` that belongs to `ctx_id`.
fn read_network_id(
    conn: &mut SqliteConnection,
    ctx_id: ContextIdBytes,
    network_name: NetworkName,
) -> Result<NetworkId, diesel::result::Error> {
    schema::networks::table
        .select(schema::networks::id)
        .filter(
            schema::networks::context_id
                .eq(ctx_id)
                .and(schema::networks::name.eq(network_name)),
        )
        .first(conn)
}

/// Reads the network setup corresponding to the given network id.
fn read_network_setup(
    conn: &mut SqliteConnection,
    network_id: NetworkId,
) -> Result<NetworkSetup, diesel::result::Error> {
    // To load the interface, machine name pairs which are connected to the network
    // we start with the enabled ports that are assigned to this network.
    // We then find the interface names by joining the enabled ports with the switch connections table
    // (recall that they share primary keys). The switch connections table has a machine_id column which
    // we should be able to find in the machine reservations table (otherwise we would not have been able to
    // set up the network if the machines are not reserved for this context). Hence we join the table again
    // with the machine reservations table to get access to the machine names as well.
    let data: Vec<(MachineNetworkInterface, MachineName)> = schema::enabled_ports::table
        .inner_join(
            schema::switch_connections::table
                .on(schema::enabled_ports::id.eq(schema::switch_connections::id)),
        )
        .inner_join(
            schema::machine_reservations::table
                .on(schema::switch_connections::machine_id.eq(schema::machine_reservations::id)),
        )
        .filter(schema::enabled_ports::net_id.eq(network_id))
        .select((
            schema::switch_connections::interface,
            schema::machine_reservations::machine_name,
        ))
        .load(conn)?;

    Ok(data
        .into_iter()
        .map(|(interface, machine_name)| TaggedMachineNetworkInterface {
            interface,
            machine_name,
        })
        .collect())
}

/// Find the network id of the given network belonging to the context.
fn lookup_network_id(
    conn: &mut SqliteConnection,
    ctx_id: ContextIdBytes,
    network_name: &NetworkNameStr,
) -> Result<NetworkId, diesel::result::Error> {
    schema::networks::table
        .select(schema::networks::id)
        .filter(
            schema::networks::context_id
                .eq(ctx_id)
                .and(schema::networks::name.eq(network_name)),
        )
        .first(conn)
}
