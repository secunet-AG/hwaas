// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::context_manager::ContextAccessToken;
use crate::path_params::PathParamsMachineName;
use crate::remote_client::reqwest_to_axum_response;
use crate::single_context_api::websocket::{connect_websockets, create_websocket};
use crate::single_context_api::{
    ContextManagerTx, DrivesApiState, GuardedContext, MachineApiState,
};
use aide::axum::{routing::get, ApiRouter};
use axum::body::Bytes;
use axum::extract::{FromRef, FromRequestParts, Path, State, WebSocketUpgrade};
use axum::http::uri::Scheme;
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use axum::response::Response;
use axum::Json;
use context_data_structures::aliases::{DriveName, MachineName};
use db_interaction::connection::DbFacade;
use db_interaction::models::aliases::{MachineId, RemoteAddress};
use db_interaction::models::context_id::ContextIdBytes;
use db_interaction::models::drives::Drive;
use db_interaction::schema;
use diesel::SelectableHelper;
use diesel::{BoolExpressionMethods, ExpressionMethods};
use diesel::{QueryDsl, RunQueryDsl};
use error_utils::{log_err, log_then_replace_err};
use image_api::sha256hash::Sha256Hash;
use machine_ops_lib::machine_data::RemoteUsbBaseUrl;
use remote_usb::usb_config::{UsbConfig, UsbFunctionConfig, UsbFunctionInfo};
use std::collections::HashMap;
use tracing::{debug, error, instrument};

use super::UsbEndpointSpecialization;

pub fn get_usb_router<S>() -> ApiRouter<S>
where
    S: Send + Sync + Clone + 'static,
    DrivesApiState: FromRef<S>,
    MachineApiState: FromRef<S>,
    GuardedContext: FromRequestParts<S>,
    ContextManagerTx: FromRef<S>,
{
    ApiRouter::new().route(
        "/usb",
        get(handle_get_usb)
            .put(handle_put_usb)
            .delete(handle_delete_usb),
    )
}

/// Lookup the reserved machine's database id and remote-usb address from the database.
async fn lookup_machine_usb(
    db_facade: &DbFacade,
    machine_name: MachineName,
    context_id: ContextIdBytes,
) -> Result<(MachineId, RemoteUsbBaseUrl), (StatusCode, &'static str)> {
    let user_facing_db_error = (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Could not load machine data from the database",
    );
    let (machine_id, remote_usb) = db_facade
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
            let remote_usb: RemoteAddress = schema::machines::table
                .find(id)
                .select(schema::machines::remote_usb)
                .first(conn)
                .inspect_err(|_| error!("could not find machine"))?;
            diesel::QueryResult::Ok((id, remote_usb))
        })
        .await
        .map_err(log_then_replace_err!(
            "could not extract machine from the database",
            user_facing_db_error
        ))?;
    let base_usb_uri: RemoteUsbBaseUrl = remote_usb
        .try_into()
        .inspect_err(log_err!(
            "BUG: invalid base usb url extracted from the database"
        ))
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal BUG detected. A bad usb url was found. Please contact the maintainers!",
            )
        })?;
    Ok((machine_id, base_usb_uri))
}

/// Proxy GET to remote-usb
#[instrument(skip(dependencies, headers))]
pub(crate) async fn handle_get_usb(
    GuardedContext(context_access_token): GuardedContext,
    State(dependencies): State<MachineApiState>,
    Path(PathParamsMachineName { machine_name }): Path<PathParamsMachineName>,
    mut headers: HeaderMap,
) -> Result<Json<Vec<UsbFunctionInfo>>, (StatusCode, String)> {
    debug!(%machine_name, "handle get usb request");

    let context_id = context_access_token.context_id;
    let db_facade = dependencies.db_facade;
    let (machine_id, remote_usb) = lookup_machine_usb(&db_facade, machine_name, context_id)
        .await
        .map_err(|(code, msg)| (code, msg.to_string()))?;
    hunt::inject_headers(&mut headers);

    let res = dependencies
        .remote_client
        .send_remote_request(
            Method::GET,
            Uri::from(remote_usb),
            machine_id,
            headers,
            None,
            false,
        )
        .await
        .map_err(|(code, msg)| (code, msg.to_string()))?;
    if res.status() != StatusCode::OK {
        error!(status = %res.status(), "remote-usb get not ok");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("HTTP {} from remote-usb", res.status()),
        ));
    }

    let mut infos: Vec<UsbFunctionInfo> = res.json().await.map_err(|e| {
        error!(error.msg = %e, error.dbg = ?e, "could not deserialize usb config");
        (
            StatusCode::BAD_GATEWAY,
            "error deserializing usb_config".to_string(),
        )
    })?;

    let context_drives = match load_context_drives_map(&db_facade, context_access_token).await {
        Ok(drives) => drives,
        Err((code, msg)) => return Err((code, msg.to_string())),
    };

    hashes_to_drives(&context_drives, &mut infos).map_err(|(code, msg)| (code, msg.to_string()))?;
    Ok(Json(infos))
}

/// Proxy PUT to remote-usb
#[instrument(skip(dependencies, headers))]
pub(crate) async fn handle_put_usb(
    GuardedContext(context_access_token): GuardedContext,
    State(dependencies): State<MachineApiState>,
    Path(PathParamsMachineName { machine_name }): Path<PathParamsMachineName>,
    mut headers: HeaderMap,
    Json(mut usb_config): Json<UsbConfig>,
) -> Result<Json<Vec<UsbFunctionInfo>>, (StatusCode, &'static str)> {
    debug!(%machine_name, "handle put usb request");

    let context_id = context_access_token.context_id;
    let db_facade = dependencies.db_facade;
    let (machine_id, remote_usb) = lookup_machine_usb(&db_facade, machine_name, context_id).await?;
    hunt::inject_headers(&mut headers);

    let context_drives = match load_context_drives_map(&db_facade, context_access_token).await {
        Ok(drives) => drives,
        Err(e) => return Err(e),
    };
    drives_to_hashes(&context_drives, &mut usb_config.functions)?;
    let body = serde_json::to_vec(&usb_config).map_err(|e| {
        error!(error = %e, "serialize usb_config");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "error serializing usb_config",
        )
    })?;

    debug!(?body, "USB config body");

    let res = dependencies
        .remote_client
        .send_remote_request(
            Method::PUT,
            Uri::from(remote_usb),
            machine_id,
            // TODO: FIXME: We simply pass the header form the user here.
            // This could lead to some unexpected behaviour (e.g. wrong content type, ...)
            headers,
            Some(body.into()),
            false,
        )
        .await?;
    if res.status() != StatusCode::OK {
        error!(status = %res.status(), "remote-usb put not ok");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "No HTTP 200 from remote-usb",
        ));
    }

    let mut infos: Vec<UsbFunctionInfo> = res.json().await.map_err(|e| {
        error!(error = %e, "deserialize usb_config");
        (StatusCode::BAD_GATEWAY, "error deserializing usb_config")
    })?;
    hashes_to_drives(&context_drives, &mut infos)?;
    Ok(Json(infos))
}

/// Proxy DELETE to remote-usb
#[instrument(skip(dependencies, headers))]
pub(crate) async fn handle_delete_usb(
    GuardedContext(context_access_token): GuardedContext,
    State(dependencies): State<MachineApiState>,
    Path(PathParamsMachineName { machine_name }): Path<PathParamsMachineName>,
    mut headers: HeaderMap,
) -> Result<(), (StatusCode, String)> {
    debug!(%machine_name, "handle delete usb request");

    let context_id = context_access_token.context_id;
    let db_facade = dependencies.db_facade;
    let (machine_id, remote_usb) = lookup_machine_usb(&db_facade, machine_name, context_id)
        .await
        .map_err(|(code, msg)| (code, msg.to_string()))?;
    hunt::inject_headers(&mut headers);

    let res = dependencies
        .remote_client
        .send_remote_request(
            Method::DELETE,
            Uri::from(remote_usb),
            machine_id,
            headers,
            None,
            false,
        )
        .await
        .map_err(|(code, msg)| (code, msg.to_string()))?;
    if res.status() == StatusCode::OK {
        Ok(())
    } else {
        error!(status = %res.status(), "remote-usb delete not ok");
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!(
                "Expected {}, but got HTTP {} from remote-usb",
                StatusCode::OK,
                res.status()
            ),
        ))
    }
}

fn drives_to_hashes(
    drives: &HashMap<String, Sha256Hash>,
    functions: &mut [UsbFunctionConfig],
) -> Result<(), (StatusCode, &'static str)> {
    for function in functions {
        if let UsbFunctionConfig::Storage { ref mut luns } = function {
            for lun in luns {
                let drive = &lun.path;
                let hash = drives
                    .get(drive)
                    .ok_or((StatusCode::NOT_FOUND, "No such drive"))?;
                lun.path.clone_from(&hash.0);
            }
        }
    }

    Ok(())
}

fn hashes_to_drives(
    drives: &HashMap<String, Sha256Hash>,
    functions: &mut [UsbFunctionInfo],
) -> Result<(), (StatusCode, &'static str)> {
    for function in functions {
        if let UsbFunctionInfo::Storage { ref mut luns } = function {
            for lun in luns {
                let hash = Sha256Hash(lun.path.clone());
                let Some((drive, _)) = drives.iter().find(|(_k, v)| **v == hash) else {
                    return Err((StatusCode::NOT_FOUND, "No such drive"));
                };
                lun.path.clone_from(drive);
            }
        }
    }

    Ok(())
}

/// Returns a hashmap between user chosen drive names and a verified hash value used as the
/// database identifier. All drives belonging to the given context_id are included.
///
/// The values are loaded from the database.
#[instrument(skip(db_facade))]
async fn load_context_drives_map(
    db_facade: &DbFacade,
    context_access_token: ContextAccessToken,
) -> Result<HashMap<DriveName, Sha256Hash>, (StatusCode, &'static str)> {
    let context_id = context_access_token.context_id;
    let user_facing_db_error = (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Could not load drives from the database",
    );
    let drives: Vec<Drive> = db_facade
        .spawn_call(move |conn| {
            db_interaction::schema::drives::table
                .select(Drive::as_select())
                .filter(db_interaction::schema::drives::context_id.eq(context_id))
                .load(conn)
        })
        .await
        .map_err(log_then_replace_err!(
            "could not load drives belonging to the context",
            user_facing_db_error
        ))?;
    let mut map = HashMap::with_capacity(drives.len());
    for drive in drives {
        let Drive { id, name, .. } = drive;
        let hash = Sha256Hash::new(id)
            .inspect_err(|e| error!(error.msg = %e, "BUG: drive identifier does not correspond to a valid hash value"))
            .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR, "An unexpected error occurred. Please contact the maintainers"))?;
        map.insert(name, hash);
    }

    Ok(map)
}

/// The handle_usb_specialization function is the entrypoint
/// for various remote_usb functionality.
///
/// It determines if the request should proxy to the remote-hands
/// service via REST or WSS connection.
#[allow(clippy::too_many_arguments)]
#[instrument(skip(dependencies, method, headers, body))]
pub(crate) async fn handle_usb_specialization(
    GuardedContext(context_access_token): GuardedContext,
    State(dependencies): State<MachineApiState>,
    Path(PathParamsMachineName { machine_name }): Path<PathParamsMachineName>,
    UsbEndpointSpecialization { specialization, .. }: UsbEndpointSpecialization,
    method: Method,
    ws_upgrade: Option<WebSocketUpgrade>,
    mut headers: HeaderMap,
    body: Option<Bytes>,
) -> Result<Response, (StatusCode, &'static str)> {
    debug!(
        %method, %machine_name, %specialization,
        "handle usb request for machine",
    );

    hunt::inject_headers(&mut headers);

    let context_id = context_access_token.context_id;
    let db_facade = dependencies.db_facade;
    let (machine_id, usb_base_uri) =
        lookup_machine_usb(&db_facade, machine_name, context_id).await?;

    let url = usb_base_uri
        .with_specialization(&specialization)
        .inspect_err(log_err!(
            "could not join given path to obtain a remote usb url"
        ))
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid url path"))?;

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
