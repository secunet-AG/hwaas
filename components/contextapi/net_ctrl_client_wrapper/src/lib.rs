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

/// Errors from sending requests to the NetCtrl API
#[derive(Debug, displaydoc::Display, thiserror::Error)]
#[non_exhaustive]
pub enum RequestError {
    /// failed to send request to NetCtrl
    InvalidRequest,

    /// failed to obtain valid response from NetCtrl
    InvalidResponse,
}

/// A high-level wrapper for the auto generated net_ctrl client.
#[derive(Clone)]
pub struct NetCtrlClient {
    base_path: String,
    client: reqwest_middleware::ClientWithMiddleware,
}

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

    #[tracing::instrument(skip(self))]
    async fn handle_request<J, T>(
        &self,
        uri: String,
        method: reqwest::Method,
        payload: Option<J>,
    ) -> Result<T, RequestError>
    where
        J: serde::Serialize + std::fmt::Debug,
        T: serde::de::DeserializeOwned,
    {
        let mut builder = self.client.request(method, &uri);
        if let Some(ref inner) = payload {
            builder = builder.json(inner);
        }

        let req = builder.build().map_err(|error| {
            error!(?error, "failed to assemble valid API request");
            RequestError::InvalidRequest
        })?;
        let resp = self.client.execute(req).await.map_err(|error| {
            error!(?error, "failed to perform API request");
            RequestError::InvalidRequest
        })?;
        let resp = resp.error_for_status().map_err(|error| {
            error!(?error, "got invalid response from NetCtrl API");
            RequestError::InvalidResponse
        })?;
        tracing::debug!(?resp, "NetCtrl API request successful");

        let value = resp.json::<T>().await.map_err(|error| {
            error!(?error, "failed to serialize NetCtrl response to user type");
            RequestError::InvalidResponse
        })?;

        Ok(value)
    }

    /// Connect the switch port to the network identified by `net_id`.
    pub async fn enable_port(
        &self,
        switch_port: &db_interaction::models::machines::SwitchPort,
        net_id: db_interaction::models::aliases::NetworkId,
    ) -> Result<(), RequestError> {
        let uri_str = format!(
            "{}/switches/{switch_id}/ports/{port_id}",
            self.base_path,
            port_id = switch_port.port,
            switch_id = switch_port.switch
        );
        let vlan_id: VlanId = VlanId::new(net_id.into());

        self.handle_request(uri_str, reqwest::Method::PUT, Some(vlan_id))
            .await
    }

    /// Disconnect the switch port.
    pub async fn disable_port(
        &self,
        switch_port: &db_interaction::models::machines::SwitchPort,
    ) -> Result<(), RequestError> {
        let uri_str = format!(
            "{}/switches/{switch_id}/ports/{port_id}",
            self.base_path,
            port_id = switch_port.port,
            switch_id = switch_port.switch
        );

        self.handle_request(uri_str, reqwest::Method::DELETE, None::<()>)
            .await
    }
}
