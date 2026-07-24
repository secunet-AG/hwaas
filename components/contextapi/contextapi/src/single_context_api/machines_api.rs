// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use crate::path_params::PathParamsMachineName;
use crate::remote_client::RemoteClient;

use super::{ContextManagerTx, DrivesApiState, GuardedContext};
use crate::single_context_api::remote_api::get_machine_remote_api_router;
use aide::axum::{routing::get_with, ApiRouter};
use aide::transform::{TransformOperation, TransformPathItem};
use axum::body::Body;
use axum::extract::{FromRef, FromRequestParts, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;

use context_data_structures::aliases::{MachineName, MachineNetworkInterface};
use context_data_structures::machine_properties::MachineInfo;
use db_interaction::connection::DbFacade;
use db_interaction::models::aliases::MachineId;
use db_interaction::models::contexts::ContextIdentifier;
use db_interaction::models::machines::{Machine, MachineReservation};
use db_interaction::schema;
use diesel::prelude::*;

use db_interaction::models::context_id::ContextIdBytes;
use error_utils::log_then_replace_err;
use serde_json::{json, Value};
use tracing::{error, info};

#[derive(Clone)]
pub(crate) struct MachineApiState {
    pub(crate) db_facade: Arc<DbFacade>,
    pub(crate) remote_client: RemoteClient,
}

impl MachineApiState {
    pub(crate) fn new(db_facade: Arc<DbFacade>, remote_client: RemoteClient) -> Self {
        Self {
            db_facade,
            remote_client,
        }
    }
}

/// Build and return the Router responsible for machine operations
pub(crate) fn get_machine_api_router<S>(request_limit: usize, state: S) -> ApiRouter<S>
where
    S: Send + Sync + Clone + 'static,
    MachineApiState: FromRef<S>,
    GuardedContext: FromRequestParts<S>,
    DrivesApiState: FromRef<S>,
    ContextManagerTx: FromRef<S>,
{
    let single_machine_api = ApiRouter::new()
        .api_route_with(
            "/",
            get_with(get_machine_info, api_doc_machines_get_info),
            api_doc_machine_api,
        )
        .api_route_with(
            "/network-interfaces",
            get_with(
                handle_get_machine_network_ports_request,
                api_doc_machine_get_net_interface_list,
            ),
            api_doc_machine_api,
        )
        .api_route_with(
            "/mjpeg",
            get_with(handle_mjpeg_request, api_doc_machine_mjpeg),
            api_doc_machine_api,
        )
        .merge(get_machine_remote_api_router(request_limit, state));

    ApiRouter::new()
        .api_route_with(
            "/",
            get_with(get_machines, api_doc_machines_list),
            api_doc_machine_api,
        )
        .nest("/:machine_name", single_machine_api)
}

/// Append OpenAPI tag to all operations
fn api_doc_machine_api(op: TransformPathItem) -> TransformPathItem {
    op.tag("Machines API")
}

/// Append OpenAPI for list machines operation
fn api_doc_machines_list(op: TransformOperation) -> TransformOperation {
    op.summary("List machines within the context")
        .description("Get all machine names contained within this context")
        .response_with::<200, Json<Vec<MachineName>>, _>(|op| {
            op.description("JSON representing all machine names as Array")
        })
}

fn api_doc_machines_get_info(op: TransformOperation) -> TransformOperation {
    op.summary("Get machine information")
        .description("Get machine information for given machine name")
        .response_with::<200, Json<MachineInfo>, _>(|op| {
            op.description("JSON representing machine info")
        })
}

/// Append OpenAPI documentation to the operation for getting all network interface names of a machines
fn api_doc_machine_get_net_interface_list(op: TransformOperation) -> TransformOperation {
    op.summary("network interface names")
        .description("List machine's network interface names.")
        .response_with::<200, Json<Vec<MachineNetworkInterface>>, _>(|op| {
            op.description("JSON representing all interface names as Array")
        })
        .response_with::<404, &str, _>(|op| op.description("Context or machine is unknown"))
}

/// Append OpenAPI documentation to the operation for getting a lightweight mjpeg stream
fn api_doc_machine_mjpeg(op: TransformOperation) -> TransformOperation {
    op.description("Establish a lightweight mjpeg stream for machine v4l output.")
        .summary("Connect to MJPEG stream via REST")
        .response_with::<200, (), _>(|op| {
            op.description(
                "Continous multipart/x-mixed-replace stream. \
            This can be used to stream an MJPEG where each frame is a valid image.",
            )
        })
}

/// Handler to obtain all names of network interfaces of one machine
#[tracing::instrument(skip(dependencies))]
async fn handle_get_machine_network_ports_request(
    GuardedContext(context_access_token): GuardedContext,
    State(dependencies): State<MachineApiState>,
    Path(PathParamsMachineName { machine_name }): Path<PathParamsMachineName>,
) -> Result<Json<Vec<MachineNetworkInterface>>, (StatusCode, &'static str)> {
    let context_id = context_access_token.context_id;
    let user_facing_db_error = (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Could not load machine data",
    );

    let network_interfaces: Vec<String> = dependencies
        .db_facade
        .spawn_call(move |conn| {
            let machine_id: MachineId = schema::machine_reservations::table
                .select(schema::machine_reservations::id)
                .filter(
                    schema::machine_reservations::context_id
                        .eq(context_id)
                        .and(schema::machine_reservations::machine_name.eq(machine_name)),
                )
                .first(conn)
                .inspect_err(|_| error!("could not load machine id from the database"))?;

            schema::switch_connections::table
                .select(schema::switch_connections::interface)
                .filter(schema::switch_connections::machine_id.eq(machine_id))
                .load(conn)
                .inspect_err(|_| {
                    error!("could not load interface names from the switch connections table")
                })
        })
        .await
        .map_err(log_then_replace_err!(
            "could not load network interfaces",
            user_facing_db_error
        ))?;

    Ok(Json(network_interfaces))
}

/// handler to obtain all machine names available within the context
#[tracing::instrument(skip(dependencies))]
async fn get_machines(
    GuardedContext(context_access_token): GuardedContext,
    State(dependencies): State<MachineApiState>,
) -> Result<Json<Value>, StatusCode> {
    let error = StatusCode::INTERNAL_SERVER_ERROR;
    let ctx_id = context_access_token.context_id;
    let machine_names: Vec<MachineName> = dependencies
        .db_facade
        .spawn_call(move |conn| {
            MachineReservation::belonging_to(&ContextIdentifier::from(ctx_id))
                .select(schema::machine_reservations::machine_name)
                .load(conn)
        })
        .await
        .map_err(log_then_replace_err!(
            "failed to load machines belonging to context",
            error
        ))?;

    Ok(Json(json!(machine_names)))
}

async fn get_machine(
    ctx_id: ContextIdBytes,
    dependencies: MachineApiState,
    machine_name: MachineName,
) -> Result<Machine, StatusCode> {
    const ERROR: StatusCode = StatusCode::INTERNAL_SERVER_ERROR;
    dependencies
        .db_facade
        .spawn_call(move |conn: &mut SqliteConnection| {
            let machine_id: MachineId = schema::machine_reservations::table
                .select(schema::machine_reservations::id)
                .filter(
                    schema::machine_reservations::context_id
                        .eq(ctx_id)
                        .and(schema::machine_reservations::machine_name.eq(machine_name)),
                )
                .first(conn)
                .inspect_err(|_| error!("could not load machine id from the database"))?;

            schema::machines::table
                .select(Machine::as_select())
                .filter(schema::machines::id.eq(machine_id))
                .first(conn)
                .inspect_err(|_| error!("could not load machine info from the machines table"))
        })
        .await
        .map_err(log_then_replace_err!(
            "failed to load info related to machine name",
            ERROR
        ))
}

/// handler to get machine information to a given machine name
#[tracing::instrument(skip(dependencies))]
async fn get_machine_info(
    GuardedContext(context_access_token): GuardedContext,
    State(dependencies): State<MachineApiState>,
    Path(PathParamsMachineName { machine_name }): Path<PathParamsMachineName>,
) -> Result<Json<Value>, StatusCode> {
    let machine: Machine =
        get_machine(context_access_token.context_id, dependencies, machine_name).await?;
    let machine_info = MachineInfo {
        id: machine.id,
        platform: machine.platform,
    };
    Ok(Json(json!(machine_info)))
}

#[tracing::instrument(skip(dependencies))]
async fn handle_mjpeg_request(
    GuardedContext(context_access_token): GuardedContext,
    State(dependencies): State<MachineApiState>,
    Path(PathParamsMachineName { machine_name }): Path<PathParamsMachineName>,
) -> Response {
    info!("Starting mjpeg stream for machine {:?}", machine_name);

    let machine: Machine = match get_machine(
        context_access_token.context_id,
        dependencies.clone(),
        machine_name,
    )
    .await
    {
        Ok(inner) => inner,
        Err(e) => {
            error!("Could not succesfully fetch machine: {:?}", e);
            return (StatusCode::NOT_FOUND, "Machine not found").into_response();
        }
    };

    let mjpg = match machine.remote_mjpeg {
        Some(inner) => inner,
        None => {
            return (
                StatusCode::NOT_FOUND,
                "Machine does not posses mjepg url. Perhaps it does not have V4L setup?",
            )
                .into_response()
        }
    };

    let client = dependencies.remote_client.client();

    let upstream = match client.get(mjpg).send().await {
        Ok(r) => r,
        Err(e) => {
            error!("Could not establish MJPEG client stream: {:?}", e);
            return (
                StatusCode::BAD_GATEWAY,
                "Stream unreachable. Perhaps the machine does not have V4L configured?",
            )
                .into_response();
        }
    };

    let mut headers = HeaderMap::new();

    if let Some(ct) = upstream.headers().get("content-type") {
        headers.insert("content-type", ct.clone());
    }

    let upstream_stream = upstream.bytes_stream();

    let body = Body::from_stream(upstream_stream);

    (headers, body).into_response()
}
