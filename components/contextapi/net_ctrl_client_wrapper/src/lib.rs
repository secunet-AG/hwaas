// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use hunt::ReqwestInjectMiddleware;
use reqwest_middleware::ClientBuilder;
use serde::{Deserialize, Serialize};
use tracing::error;

/// VlanId : Wrapper type for different VLAN IDs potentially used for different use cases.
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct VlanId {
    /// Standard 802.1q ID.
    #[serde(rename = "vlan_id")]
    pub vlan_id: i32,
}

impl VlanId {
    /// Wrapper type for different VLAN IDs potentially used for different use cases.
    pub fn new(vlan_id: i32) -> VlanId {
        VlanId { vlan_id }
    }
}

/// A high-level wrapper for the auto generated net_ctrl client.
#[derive(Clone)]
pub struct NetCtrlClient {
    base_path: String,
    client: reqwest_middleware::ClientWithMiddleware,
}

/// failed to enable switch port {port_id}: {reason}
#[derive(Debug, displaydoc::Display)]
pub struct EnablePortError {
    port_id: String,
    reason: String,
}

impl std::error::Error for EnablePortError {}

/// failed to disable switch port {port_id}: {reason}
#[derive(Debug, displaydoc::Display)]
pub struct DisablePortError {
    port_id: String,
    reason: String,
}

impl std::error::Error for DisablePortError {}

impl NetCtrlClient {
    /// Construct a new net ctrl client with the given base path.
    pub fn new(net_ctrl_base_path: String) -> Self {
        Self {
            base_path: net_ctrl_base_path,
            client: ClientBuilder::new(Default::default())
                .with(ReqwestInjectMiddleware)
                .build(),
        }
    }

    /// Connect the switch port to the network identified by `net_id`.
    pub async fn enable_port(
        &self,
        switch_port: &db_interaction::models::machines::SwitchPort,
        net_id: db_interaction::models::aliases::NetworkId,
    ) -> Result<(), EnablePortError> {
        let user_error = |reason: &'static str| EnablePortError {
            port_id: switch_port.port.clone(),
            reason: reason.to_string(),
        };

        let uri_str = format!(
            "{}/switches/{switch_id}/ports/{port_id}",
            self.base_path,
            port_id = switch_port.port,
            switch_id = switch_port.switch
        );
        let vlan_id: VlanId = VlanId::new(net_id.into());

        let req = self
            .client
            .request(reqwest::Method::PUT, &uri_str)
            .json(&vlan_id)
            .build()
            .map_err(|error| {
                error!(?error, ?switch_port, "failed to assemble API request");
                user_error("failed to communicate with NetCtrl API")
            })?;
        let resp = self.client.execute(req).await.map_err(|error| {
            error!(
                ?error,
                ?switch_port,
                "failed to send API request to NetCtrl"
            );
            user_error("failed to communicate with NetCtrl API")
        })?;

        let status = resp.error_for_status().map_err(|error| {
            error!(?error, ?switch_port, "received error response from NetCtrl");
            user_error("received invalid API response")
        })?;
        tracing::debug!(?status, "NetCtrl API request successful");

        Ok(())
    }

    /// Disconnect the switch port.
    pub async fn disable_port(
        &self,
        switch_port: &db_interaction::models::machines::SwitchPort,
    ) -> Result<(), DisablePortError> {
        let user_error = |reason: &'static str| DisablePortError {
            port_id: switch_port.port.clone(),
            reason: reason.to_string(),
        };

        let uri_str = format!(
            "{}/switches/{switch_id}/ports/{port_id}",
            self.base_path,
            port_id = switch_port.port,
            switch_id = switch_port.switch
        );

        let req = self
            .client
            .request(reqwest::Method::DELETE, &uri_str)
            .build()
            .map_err(|error| {
                error!(?error, ?switch_port, "failed to assemble API request");
                user_error("failed to communicate with NetCtrl API")
            })?;
        let resp = self.client.execute(req).await.map_err(|error| {
            error!(
                ?error,
                ?switch_port,
                "failed to send API request to NetCtrl"
            );
            user_error("failed to communicate with NetCtrl API")
        })?;

        let status = resp.error_for_status().map_err(|error| {
            error!(?error, ?switch_port, "received error response from NetCtrl");
            user_error("received invalid API response")
        })?;
        tracing::debug!(?status, "NetCtrl API request successful");
        Ok(())
    }
}
