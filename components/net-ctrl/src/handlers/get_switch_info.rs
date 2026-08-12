// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use aide::transform::TransformOperation;
use axum::Json;
use axum::extract::{Path, State};
use std::sync::Arc;
use tracing::instrument;

use crate::connection_handler::ConnectionHandler;
use crate::handlers::PathParamsSwitchID;
use crate::handlers::external_api_errors::ExtApiError;
use crate::network_type_ids::PortRepresentation;
use crate::switch::SwitchAPI;

/// Requests information about a single Switch.
///
/// Returns list of PortRepresentations, containing all required info about all ports of a switch
#[instrument(skip(connection_handler))]
pub async fn get_switch_info(
    State(connection_handler): State<Arc<ConnectionHandler>>,
    Path(PathParamsSwitchID { switch_id }): Path<PathParamsSwitchID>,
) -> Result<Json<Vec<PortRepresentation>>, ExtApiError> {
    // get correct API for this switch
    let api = connection_handler.get_switch_api(&switch_id)?;
    Ok(Json(api.get_ports().await?))
}

/// Append OpenAPI for list machines operation
pub(crate) fn api_doc_get_switch_info(op: TransformOperation) -> TransformOperation {
    op.summary("switches details")
        .description("Get information about a switch")
        .response_with::<200, Json<Vec<PortRepresentation>>, _>(|op| {
            op.description("JSON representing all machine names as Array")
        })
        .response_with::<404, (), _>(|op| op.description("Switch not found."))
        .response_with::<500, (), _>(|op| op.description("Could not get switch API"))
        .response_with::<410, (), _>(|op| {
            op.description("There is currently no switch api. Please retry")
        })
}
