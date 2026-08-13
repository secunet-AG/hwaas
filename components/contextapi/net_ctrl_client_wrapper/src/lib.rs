// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use hunt::ReqwestInjectMiddleware;
use net_ctrl_client::apis::configuration::Configuration;
use net_ctrl_client::models::VlanId;
use reqwest_middleware::ClientBuilder;
use std::sync::Arc;

/// A high-level wrapper for the auto generated net_ctrl client.
#[derive(Clone)]
pub struct NetCtrlClient {
    pub config: Arc<Configuration>,
}

impl NetCtrlClient {
    /// Construct a new net ctrl client with the given base path.
    pub fn new(net_ctrl_base_path: String) -> Self {
        Self {
            config: Configuration {
                base_path: net_ctrl_base_path,
                client: ClientBuilder::new(Default::default())
                    .with(ReqwestInjectMiddleware)
                    .build(),
                ..Default::default()
            }
            .into(),
        }
    }

    /// Connect the switch port to the network identified by `net_id`.
    pub async fn enable_port(
        &self,
        switch_port: &db_interaction::models::machines::SwitchPort,
        net_id: db_interaction::models::aliases::NetworkId,
    ) -> Result<(), impl std::error::Error + Send + Sync + 'static> {
        // Convert the net_id to NetCtrl Type
        let vlan_id: VlanId = VlanId::new(net_id.into());
        net_ctrl_client::apis::default_api::switches_switch_id_ports_port_id_put(
            &self.config,
            &switch_port.port.clone(),
            switch_port.switch.as_str(),
            vlan_id,
        )
        .await
    }

    /// Disconnect the switch port.
    pub async fn disable_port(
        &self,
        switch_port: &db_interaction::models::machines::SwitchPort,
    ) -> Result<(), impl std::error::Error + Send + Sync + 'static + use<>> {
        net_ctrl_client::apis::default_api::switches_switch_id_ports_port_id_delete(
            &self.config,
            &switch_port.port.clone(),
            switch_port.switch.as_str(),
        )
        .await
    }
}
