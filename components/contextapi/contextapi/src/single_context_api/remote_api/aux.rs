// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::path_params::PathParamsMachineName;
use crate::remote_client::reqwest_to_axum_response;
use crate::single_context_api::{GuardedContext, MachineApiState};
use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;

use axum::{extract::Path, http::HeaderMap, http::Method};
use context_data_structures::aliases::MachineName;
use db_interaction::connection::DbFacade;
use db_interaction::models::aliases::{MachineId, RemoteAddress};
use db_interaction::models::context_id::ContextIdBytes;
use db_interaction::schema;
use diesel::{BoolExpressionMethods, ExpressionMethods};
use diesel::{QueryDsl, RunQueryDsl};
use error_utils::log_then_replace_err;
use machine_ops_lib::machine_data::RemoteAuxiliaryBaseUrl;
use tracing::{debug, error, instrument};

use super::AuxiliaryEndpointSpecialization;

/// Lookup the reserved machine's database id and its remote-auxiliary address
async fn lookup_machine_aux(
    db_facade: &DbFacade,
    machine_name: MachineName,
    context_id: ContextIdBytes,
) -> Result<(MachineId, RemoteAuxiliaryBaseUrl), (StatusCode, &'static str)> {
    let (machine_id, remote_auxiliary) = db_facade
        .spawn_call(move |conn| {
            // TODO: Consider using a join instead.
            let machine_id: MachineId = schema::machine_reservations::table
                .select(schema::machine_reservations::id)
                .filter(
                    schema::machine_reservations::machine_name
                        .eq(machine_name)
                        .and(schema::machine_reservations::context_id.eq(context_id)),
                )
                .first(conn)
                .inspect_err(|_| error!("id lookup of reserved machine failed"))?;
            let remote_aux: Option<RemoteAddress> = schema::machines::table
                .select(schema::machines::remote_auxiliary)
                .filter(schema::machines::id.eq(machine_id))
                .first(conn)
                .inspect_err(|_| error!("remote auxiliary for machine query failed"))?;
            diesel::QueryResult::Ok((machine_id, remote_aux))
        })
        .await
        .map_err(log_then_replace_err!(
            "failed to lookup machine data",
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to lookup the address for the machine's auxiliary device service"
            )
        ))?;

    let Some(remote_auxiliary) = remote_auxiliary else {
        return Err((
            StatusCode::NOT_FOUND,
            "No auxiliary devices attached to this machine",
        ));
    };
    let remote_auxiliary: RemoteAuxiliaryBaseUrl =
        remote_auxiliary.try_into().map_err(log_then_replace_err!(
            "BUG: invalid remote auxiliary address stored in the database",
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Invalid auxiliary address in the database. Please contact the maintainers!"
            )
        ))?;

    Ok((machine_id, remote_auxiliary))
}

/// Proxy a request to remote-auxiliary
#[allow(clippy::too_many_arguments)]
#[instrument(skip(dependencies, method, headers, body))]
pub(crate) async fn handle_aux_specialization(
    GuardedContext(context_access_token): GuardedContext,
    State(dependencies): State<MachineApiState>,
    Path(PathParamsMachineName { machine_name }): Path<PathParamsMachineName>,
    AuxiliaryEndpointSpecialization { specialization, .. }: AuxiliaryEndpointSpecialization,
    method: Method,
    mut headers: HeaderMap,
    body: Option<Bytes>,
) -> Result<Response, (StatusCode, &'static str)> {
    debug!(
    %method, %machine_name, %specialization,
        "handle aux request for machine",
    );

    hunt::inject_headers(&mut headers);

    let context_id = context_access_token.context_id;
    let db_facade = dependencies.db_facade;

    let (machine_id, remote_aux) = lookup_machine_aux(&db_facade, machine_name, context_id).await?;

    let url = remote_aux
        .with_specialization(&specialization)
        .map_err(log_then_replace_err!(
            "could not join the given path to obtain a remote auxiliary url",
            (StatusCode::BAD_REQUEST, "Invalid url")
        ))?;

    let response = dependencies
        .remote_client
        .send_remote_request(method, url, machine_id, headers, body, true)
        .await?;
    reqwest_to_axum_response(response)
}

/// Proxy a request to the root of a known remote-auxiliary instance.
#[instrument(skip(dependencies, method, headers, body))]
pub(crate) async fn handle_aux_root(
    GuardedContext(context_access_token): GuardedContext,
    State(dependencies): State<MachineApiState>,
    Path(PathParamsMachineName { machine_name }): Path<PathParamsMachineName>,
    method: Method,
    mut headers: HeaderMap,
    body: Option<Bytes>,
) -> Result<Response, (StatusCode, &'static str)> {
    debug!(
    %method, %machine_name,
        "handle aux request for machine",
    );

    hunt::inject_headers(&mut headers);

    let context_id = context_access_token.context_id;
    let db_facade = dependencies.db_facade;

    let (machine_id, remote_aux) = lookup_machine_aux(&db_facade, machine_name, context_id).await?;

    let response = dependencies
        .remote_client
        .send_remote_request(method, remote_aux.into(), machine_id, headers, body, true)
        .await?;
    reqwest_to_axum_response(response)
}
