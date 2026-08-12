// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use aide::transform::TransformOperation;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use std::sync::Arc;
use tracing::{debug, instrument, warn};

use crate::connection_handler::ConnectionHandler;
use crate::handlers::PathParamsSwitchAndPortID;
use crate::handlers::external_api_errors::ExtApiError;
use crate::network_type_ids::{PortID, SwitchID, VlanID};
use crate::switch::SwitchAPI;

/// Adds the port as untagged with the given port_id to the Vlan with the given vlan_id on the switch identified by switch_id.
///
/// Returns an Error when Vlan is locked from receiving new members.
#[instrument(skip(connection_handler))]
pub async fn enable_port(
    State(connection_handler): State<Arc<ConnectionHandler>>,
    Path(PathParamsSwitchAndPortID { switch_id, port_id }): Path<PathParamsSwitchAndPortID>,
    Json(vlan_id): Json<VlanID>,
) -> Result<(), ExtApiError> {
    catch_forbidden_ports(connection_handler.clone(), &switch_id, &port_id).await?;
    // get correct API for this switch
    let api = connection_handler.get_switch_api(&switch_id)?;
    // enable the port on the switch and add it to the specified VLAN
    api.add_untagged_port(&vlan_id, &port_id)
        .await
        .map_err(|e| e.into())
}

/// Append OpenAPI for list machines operation
pub(crate) fn api_doc_enable_port(op: TransformOperation) -> TransformOperation {
    op.summary("enable port")
        .description("Enable a switch port and assign a VLAN")
        .response_with::<200, (), _>(|op| {
            op.description("VLAN is assigned and port is enabled")
        })
        .response_with::<401, (), _>(|op| {
            op.description("This port is protected. Please check if you supplied the correct ID.",)
        })
        .response_with::<404, (), _>(|op| {
            op.description( "Switch or its port was not found.")
        })
        .response_with::<400, (), _>(|op| {
            op.description("Expected request body to contain a map with field vlan_id from interval [2..4093].")
        })
}

/// Disables the given port on the given switch.
///
/// Returns an Error when it is forbidden to disable this Port.
#[instrument(skip(connection_handler))]
pub(crate) async fn disable_port(
    State(connection_handler): State<Arc<ConnectionHandler>>,
    Path(PathParamsSwitchAndPortID { switch_id, port_id }): Path<PathParamsSwitchAndPortID>,
) -> Result<(), ExtApiError> {
    debug!("Handle 'disable port' request");

    // test if the port is forbidden and if so perform an early return
    catch_forbidden_ports(connection_handler.clone(), &switch_id, &port_id)
        .await
        .map_err(|e| {
            warn!("Could not disable forbidden port: {:?}", e);
            e
        })?;

    // get correct API for this switch
    let api = connection_handler.get_switch_api(&switch_id).map_err(|e| {
        warn!("Could not get Switch Backend: {:?}", e);
        e
    })?;

    // disable the port, remove it from its VLAN
    api.remove_port(&port_id).await.map_err(|e| {
        warn!("Could not disable port: {:?}", e);
        e
    })?;
    Ok(())
}

/// Append OpenAPI for list machines operation
pub(crate) fn api_doc_disable_port(op: TransformOperation) -> TransformOperation {
    op.summary("disable port")
        .description("Disable a switch port and un-assign VLAN")
        .response::<200, ()>()
        .response_with::<401, (), _>(|op| {
            op.description("This port is protected. Please check if you supplied the correct ID.")
        })
        .response_with::<404, (), _>(|op| op.description("Switch or its port was not found."))
}

async fn catch_forbidden_ports(
    connection_handler: Arc<ConnectionHandler>,
    switch_id: &SwitchID,
    port_id: &PortID,
) -> Result<(), ExtApiError> {
    // test for preset forbidden port ids
    if connection_handler
        .is_port_forbidden(switch_id, port_id)
        .await?
    {
        Err(ExtApiError::new(
            StatusCode::UNAUTHORIZED,
            "This port is protected. Please check if you supplied the correct ID.",
        ))
    } else {
        Ok(())
    }
}
