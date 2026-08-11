// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use aide::transform::TransformOperation;
use axum::Json;
use axum::extract::State;
use std::sync::Arc;
use tracing::instrument;

use crate::handlers::external_api_errors::ExtApiError;
use connection_handler::ConnectionHandler;
use connection_handler::SwitchMapping;

/// Requests a list of all Switches.
///
/// Returns list of Switch IDs and their model.
#[instrument(skip(connection_handler))]
pub(crate) async fn get_switches(
    State(connection_handler): State<Arc<ConnectionHandler>>,
) -> Result<Json<SwitchMapping>, ExtApiError> {
    connection_handler
        .get_switches()
        .await
        .map_err(|e| e.into())
        .map(Json)
}

/// Append OpenAPI for list machines operation
pub(crate) fn api_doc_get_switches(op: TransformOperation) -> TransformOperation {
    op.summary("List all switches")
        .description("Get all switch IDs from the inventory")
        .response_with::<200, Json<SwitchMapping>, _>(|op| {
            op.description("JSON representing all machine names as Array")
        })
}
