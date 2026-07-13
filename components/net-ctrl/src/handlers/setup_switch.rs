// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use aide::transform::TransformOperation;
use axum::extract::{Path, State};
use axum::Json;
use connection_handler::ConnectionHandler;
use std::sync::Arc;
use switch::SwitchAPI;
use tracing::instrument;
use tracing::{debug, warn};

use crate::handlers::external_api_errors::ExtApiError;
use crate::handlers::setup_data::{SetupData, VlanIDVec};
use crate::handlers::PathParamsSwitchID;

/// Set up a specific switch.
///
/// Returns an Error when something went wrong.
#[instrument(skip(connection_handler,setup_data), fields(vlan_id_range = format!("{:?}", setup_data.vlan_id_range)))]
pub async fn setup_switch(
    State(connection_handler): State<Arc<ConnectionHandler>>,
    Path(PathParamsSwitchID { switch_id }): Path<PathParamsSwitchID>,
    Json(setup_data): Json<SetupData>,
) -> Result<(), ExtApiError> {
    debug!("Handle 'setup_switch' request");

    // get correct API for this switch
    let api = connection_handler
        .get_switch_api(&switch_id)
        .inspect_err(|e| {
            warn!("Could not get Switch Backend: {:?}", e);
        })?;

    // transform user's range of u16 to vec of valid VLAN IDs
    let vlan_ids: VlanIDVec = setup_data.try_into().inspect_err(|_| {
        debug!("Invalid VlanID");
    })?;

    // perform the setup
    api.setup(vlan_ids.vlan_ids).await.map_err(|e| {
        warn!("Switch setup failed ({e})");
        e.into()
    })
}

/// Append OpenAPI for list machines operation
pub(crate) fn api_doc_setup_switch(op: TransformOperation) -> TransformOperation {
    op.summary("setup switch")
        .description("setup VLANs and trunk ports")
        .response_with::<200, (), _>(|op| {
            op.description("JSON representing all machine names as Array")
        })
        .response_with::<404, (), _>(|op| op.description("Switch not found."))
        .response_with::<500, (), _>(|op| op.description("Could not get switch API"))
        .response_with::<410, (), _>(|op| {
            op.description("There is currently no switch api. Please retry")
        })
}
