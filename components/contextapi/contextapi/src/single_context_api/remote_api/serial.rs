// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::path_params::PathParamsMachineName;
use crate::remote_client::reqwest_to_axum_response;
use crate::single_context_api::websocket::{connect_websockets, create_websocket};
use crate::single_context_api::{GuardedContext, MachineApiState};
use axum::body::Bytes;
use axum::extract::{State, WebSocketUpgrade};
use axum::http::uri::Scheme;
use axum::http::{StatusCode, Uri};
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
use machine_ops_lib::machine_data::RemoteSerialBaseUrl;
use tracing::{debug, error, instrument};

use super::SerialEndpointSpecialization;

/// Get the Url for the machine's remote serial
async fn lookup_machine_serial(
    db_facade: &DbFacade,
    machine_name: MachineName,
    context_id: ContextIdBytes,
) -> Result<(MachineId, RemoteSerialBaseUrl), (StatusCode, &'static str)> {
    let user_facing_db_error = (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Could not load machine data from the database",
    );
    let (machine_id, remote_serial) = db_facade
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

            let remote_serial: Option<RemoteAddress> = schema::machines::table
                .select(schema::machines::remote_serial)
                .filter(schema::machines::id.eq(machine_id))
                .first(conn)
                .inspect_err(|_| error!("remote serial for machine query failed"))?;
            Ok((machine_id, remote_serial))
        })
        .await
        .map_err(log_then_replace_err!(
            "could not extract machine from the database",
            user_facing_db_error
        ))?;

    let Some(remote_serial) = remote_serial else {
        return Err((StatusCode::NOT_FOUND, "No serial attached to this machine"));
    };

    let remote_serial: RemoteSerialBaseUrl =
        remote_serial.try_into().map_err(log_then_replace_err!(
            "BUG: invalid remote serial stored in the database",
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Invalid serial address in the database. Please contact the maintainers!"
            )
        ))?;

    Ok((machine_id, remote_serial))
}

/// Proxy a request to the root of a known remote-serial instance.
#[instrument(skip(method, headers, body, dependencies))]
pub(crate) async fn handle_serial_root(
    GuardedContext(context_access_token): GuardedContext,
    State(dependencies): State<MachineApiState>,
    Path(PathParamsMachineName { machine_name }): Path<PathParamsMachineName>,
    method: Method,
    mut headers: HeaderMap,
    body: Option<Bytes>,
) -> Result<Response, (StatusCode, &'static str)> {
    debug!(
        %method, %machine_name,
        "handle serial request for machine",
    );
    hunt::inject_headers(&mut headers);
    let context_id = context_access_token.context_id;
    let db_facade = dependencies.db_facade;
    let (machine_id, remote_serial) =
        lookup_machine_serial(&db_facade, machine_name, context_id).await?;

    let response = dependencies
        .remote_client
        .send_remote_request(
            method,
            remote_serial.into(),
            machine_id,
            headers,
            body,
            false,
        )
        .await?;
    reqwest_to_axum_response(response)
}

/// Proxy a request to a known remote-serial instance.
#[allow(clippy::too_many_arguments)]
#[instrument(skip(method, headers, body, dependencies))]
pub(crate) async fn handle_serial_specialization(
    GuardedContext(context_access_token): GuardedContext,
    State(dependencies): State<MachineApiState>,
    Path(PathParamsMachineName { machine_name }): Path<PathParamsMachineName>,
    SerialEndpointSpecialization { specialization, .. }: SerialEndpointSpecialization,
    method: Method,
    ws_upgrade: Option<WebSocketUpgrade>,
    mut headers: HeaderMap,
    body: Option<Bytes>,
) -> Result<Response, (StatusCode, &'static str)> {
    debug!(
        %method, %machine_name,
        "handle serial request for machine",
    );
    hunt::inject_headers(&mut headers);

    let context_id = context_access_token.context_id;
    let db_facade = dependencies.db_facade;
    let (machine_id, remote_serial) =
        lookup_machine_serial(&db_facade, machine_name, context_id).await?;

    let url = remote_serial
        .with_specialization(&specialization)
        .map_err(log_then_replace_err!(
            "could not join given path to obtain a remote serial url",
            (StatusCode::BAD_REQUEST, "Invalid url")
        ))?;

    // Determine whether to setup a websocket or just forward the request as-is
    // based on the value of ws_upgrade
    if let Some(wsu) = ws_upgrade {
        let mut uri_parts = url.into_parts();
        // Replace http/https with websocket URI scheme
        uri_parts.scheme = match uri_parts.scheme {
            Some(scheme) if scheme == Scheme::HTTPS => "wss".parse().ok(),
            _ => "ws".parse().ok(),
        };
        let Ok(url) = Uri::from_parts(uri_parts) else {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to appropriately modify uri scheme",
            ));
        };

        let target_websocket = create_websocket(url)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to connect "))?;

        // The websocket should be closed if the entire context is deleted.
        let context_termination_signal = context_access_token.cancelled_owned();

        return Ok(wsu.on_upgrade(move |socket_user| {
            connect_websockets(socket_user, target_websocket, context_termination_signal)
        }));
    } else {
        let response = dependencies
            .remote_client
            .send_remote_request(method, url, machine_id, headers, body, false)
            .await?;
        reqwest_to_axum_response(response)
    }
}
