// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::context_manager::ContextAccessToken;
use crate::path_params::PathParamsMachineName;
use crate::remote_client::reqwest_to_axum_response;
use crate::single_context_api::{GuardedContext, MachineApiState};
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{StatusCode, Uri};
use axum::response::Response;

use axum::{extract::Path, http::HeaderMap, http::Method};
use context_data_structures::aliases::MachineName;
use db_interaction::connection::DbFacade;
use db_interaction::models::aliases::{MachineId, RemoteAddress};
use db_interaction::models::context_id::ContextIdBytes;
use db_interaction::schema;
use diesel::{BoolExpressionMethods, ExpressionMethods, QueryDsl, RunQueryDsl};
use error_utils::{log_err, log_then_replace_err};
use machine_ops_lib::machine_data::RemotePowerBaseUrl;
use tracing::{debug, error, instrument};

use super::PowerEndpointSpecialization;

/// Lookup the reserved machine's database id and remote-power address from the database.
async fn lookup_machine_power(
    db_facade: &DbFacade,
    machine_name: MachineName,
    context_id: ContextIdBytes,
) -> Result<(MachineId, RemotePowerBaseUrl), (StatusCode, &'static str)> {
    let user_facing_db_error = (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Could not load machine data from the database",
    );
    let (machine_id, base_power_uri) = db_facade
        .spawn_call(move |conn| {
            // TODO: Consider using a join instead.
            let id: MachineId = schema::machine_reservations::table
                .select(schema::machine_reservations::id)
                .filter(
                    schema::machine_reservations::machine_name
                        .eq(machine_name)
                        .and(schema::machine_reservations::context_id.eq(context_id)),
                )
                .first(conn)
                .inspect_err(|_| error!("id lookup of reserved machine failed"))?;
            let remote_power: RemoteAddress = schema::machines::table
                .find(id)
                .select(schema::machines::remote_power)
                .first(conn)
                .inspect_err(|_| error!("could not find machine"))?;
            diesel::QueryResult::Ok((id, remote_power))
        })
        .await
        .map_err(log_then_replace_err!(
            "could not extract machine from the database",
            user_facing_db_error
        ))?;
    let base_power_uri: RemotePowerBaseUrl = base_power_uri
        .try_into()
        .inspect_err(log_err!(
            "BUG: invalid base power url extracted from the database"
        ))
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal BUG detected. A bad power url was found. Please contact the maintainers!",
            )
        })?;
    Ok((machine_id, base_power_uri))
}

/// Proxy a request to remote-power for a given power interface
#[instrument(skip(dependencies, method, headers, body))]
pub(crate) async fn handle_power_specialization(
    GuardedContext(context_access_token): GuardedContext,
    State(dependencies): State<MachineApiState>,
    Path(PathParamsMachineName { machine_name }): Path<PathParamsMachineName>,
    PowerEndpointSpecialization { specialization, .. }: PowerEndpointSpecialization,
    method: Method,
    headers: HeaderMap,
    body: Option<Bytes>,
) -> Result<Response, (StatusCode, &'static str)> {
    debug!(
        %method, %machine_name, %specialization,
        "handle power request for machine",
    );
    let strategy = |remote_power: RemotePowerBaseUrl| {
        remote_power
            .with_specialization(&specialization)
            .inspect_err(log_err!(
                "could not join given path to obtain a remote power url"
            ))
            .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid url path"))
    };
    handle_power_with_strategy(
        context_access_token,
        dependencies,
        machine_name,
        method,
        headers,
        body,
        strategy,
    )
    .await
}

/// Proxy a request to remote-power
#[instrument(skip(dependencies, method, headers, body))]
pub(crate) async fn handle_power_root(
    GuardedContext(context_access_token): GuardedContext,
    State(dependencies): State<MachineApiState>,
    Path(PathParamsMachineName { machine_name }): Path<PathParamsMachineName>,
    method: Method,
    headers: HeaderMap,
    body: Option<Bytes>,
) -> Result<Response, (StatusCode, &'static str)> {
    debug!(
        %method, %machine_name,
        "handle power request for machine",
    );
    let strategy = |remote_power: RemotePowerBaseUrl| Ok(Uri::from(remote_power));
    handle_power_with_strategy(
        context_access_token,
        dependencies,
        machine_name,
        method,
        headers,
        body,
        strategy,
    )
    .await
}

async fn handle_power_with_strategy<F>(
    context_access_token: ContextAccessToken,
    dependencies: MachineApiState,
    machine_name: String,
    method: Method,
    mut headers: HeaderMap,
    body: Option<Bytes>,
    url_strategy: F,
) -> Result<Response, (StatusCode, &'static str)>
where
    F: FnOnce(RemotePowerBaseUrl) -> Result<Uri, (StatusCode, &'static str)>,
{
    hunt::inject_headers(&mut headers);

    let context_id = context_access_token.context_id;
    let db_facade = dependencies.db_facade;
    let (machine_id, remote_power) =
        lookup_machine_power(&db_facade, machine_name, context_id).await?;
    let url = url_strategy(remote_power)?;

    let response = dependencies
        .remote_client
        .send_remote_request(method, url, machine_id, headers, body, false)
        .await?;
    reqwest_to_axum_response(response)
}
