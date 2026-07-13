// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::fs_n8550::fs_interface::{
    FSN8550InterfaceGigabitEthernetContainer,
    FSN8550InterfaceGigabitEthernetFamilyEthernetFamilyContainer,
    FSN8550InterfaceGigabitEthernetFamilyEthernetSwitchingContainer,
    FSN8550InterfaceGigabitEthernetFamilyEthernetSwitchingVlanContainer, FSN8550InterfacesModule,
    FSN8550InterfacesNode, PortMode, VlanMember,
};
use crate::fs_n8550::fs_vlans::{
    FSN8550VlansContainer, FSN8550VlansModule, FSN8550VlansVlanIdContainer,
};
use crate::{SwitchAPI, SwitchApiError, SwitchSetupError};
use async_trait::async_trait;
use base64::Engine;
use dashmap::DashSet;
use http::{header, HeaderMap};
use network_type_ids::{PortID, PortRepresentation, SwitchDetails, VlanID};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use std::sync::Arc;
use tokio::sync::{OnceCell, Semaphore};
use tracing::{debug, error, instrument, warn};
use url::Url;

#[derive(Clone)]
pub struct FSN8550 {
    /// A reqwest client for establishing a session to the switch and sending it requests.
    client: ClientWithMiddleware,

    /// Switch details containing the address and login data for a switch.
    switch_details: SwitchDetails,

    /// Base URI for all REST API calls
    root_uri: Url,

    ports: OnceCell<DashSet<PortID>>,

    /// permit to set up the trunk links and prevent inconsistency during parallel setup request
    setup_guard: Arc<Semaphore>,
}

impl FSN8550 {
    pub fn new(switch_details: SwitchDetails) -> Result<Self, SwitchApiError> {
        let credential = switch_details
            .credentials
            .clone()
            .ok_or(SwitchApiError::Unauthorized)?;

        let auth_str: String = "Basic ".to_owned()
            + &*base64::engine::general_purpose::STANDARD
                .encode(format!("{}:{}", credential.username, credential.password));

        let default_headers = HeaderMap::from_iter(vec![
            (
                header::CONTENT_TYPE,
                header::HeaderValue::from_static("application/yang-data+json"),
            ),
            (
                header::ACCEPT,
                header::HeaderValue::from_static("application/yang-data+json"),
            ),
            (
                header::AUTHORIZATION,
                header::HeaderValue::from_str(auth_str.as_str()).map_err(|e| {
                    error!(e = ?e, "Could not prepare basic auth header");
                    SwitchApiError::Unauthorized
                })?,
            ),
        ]);

        // prepare a client
        let client = reqwest::Client::builder()
            .no_proxy()
            .default_headers(default_headers)
            // FS PicOS switches serve the REST API only via HTTPS with self-signed certs
            // We do not want to manage these certs for now.
            .danger_accept_invalid_certs(true)
            .build()
            .map_err(|_| {
                error!("build reqwest client failed");
                SwitchApiError::BuiltFaultyRequestToSwitch
            })?;

        let client = ClientBuilder::new(client).build();

        let root_uri = Url::parse(&format!(
            "https://{}:{}/restconf/data/",
            &switch_details.ip,
            switch_details.port.unwrap_or(443),
        ))
        .map_err(|e| {
            warn!(error = %e, "building base URL failed");
            SwitchApiError::DestinationUnreachable
        })?;

        Ok(Self {
            switch_details,
            client,
            root_uri,
            ports: OnceCell::new(),
            setup_guard: Arc::new(Semaphore::new(1)),
        })
    }

    /// build url to reach dst with requests.
    /// The suffix describes uri of resources to access.
    #[instrument(skip(self), level = "trace")]
    fn build_url(&self, suffix: &str) -> Result<Url, SwitchApiError> {
        let mut url = self.root_uri.clone();

        url.set_path((self.root_uri.path().to_owned() + suffix).as_str());

        Ok(url)
    }

    /// get all port identifiers from the switch
    #[instrument(skip(self))]
    async fn retrieve_port_list(&self) -> Result<DashSet<PortID>, SwitchApiError> {
        let url = self.build_url("interface:interface")?;
        let res_ports = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|_| SwitchApiError::DestinationUnreachable)?;

        if !res_ports.status().is_success() {
            warn!( status = ?res_ports.status(), "could not get ports");
            return Err(SwitchApiError::UnexpectedResponseFromSwitch);
        }

        let res_ports = res_ports
            .json::<FSN8550InterfacesModule>()
            .await
            .map_err(|_| SwitchApiError::UnexpectedResponseFromSwitch)?;

        let set = DashSet::from_iter(
            res_ports
                .data
                .gigabit_ethernet
                .iter()
                .map(|i| PortID::from(i.name.clone())),
        );
        debug!(len = set.len(), "port-list constructed");
        Ok(set)
    }

    /// Alter an access port setting (do not use for trunks).
    /// If there is a `vlan_id` specified the port is enabled and the vlan is set in one step.
    /// Else the port is disabled and assigned to the configured default VLAN.
    #[instrument(skip(self))]
    async fn alter_port(
        &self,
        port_id: &PortID,
        vlan_id: Option<&VlanID>,
    ) -> Result<(), SwitchApiError> {
        let url = self
            .build_url("interface:interface")
            .map_err(|_| SwitchApiError::BuiltFaultyRequestToSwitch)?;
        let interfaces =  FSN8550InterfacesModule {
            data: FSN8550InterfacesNode {
                gigabit_ethernet: vec![
                    FSN8550InterfaceGigabitEthernetContainer {
                        name: format!("xe-{}", port_id),
                        disable: vlan_id.is_none(),
                        family: FSN8550InterfaceGigabitEthernetFamilyEthernetFamilyContainer {
                            ethernet_switching: FSN8550InterfaceGigabitEthernetFamilyEthernetSwitchingContainer {
                                native_vlan_id: vlan_id.unwrap_or(&self.switch_details.default_vlan).vlan_id(),
                                port_mode: PortMode::Access,
                                vlan: FSN8550InterfaceGigabitEthernetFamilyEthernetSwitchingVlanContainer {
                                    members: vec![]
                                },
                            }
                        },
                    }
                ]
            },
        };

        let res = self
            .client
            .patch(url)
            .header(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static("application/yang-data+json"),
            )
            .json(&interfaces)
            .send()
            .await
            .map_err(|_| SwitchApiError::UnexpectedResponseFromSwitch)?;

        if !res.status().is_success() {
            let code = res.status();
            let body: serde_json::Value = res.json().await.unwrap_or_default();
            error!(status = ?code, msg = ?body, "could not configure interface");
            return Err(SwitchApiError::UnexpectedResponseFromSwitch);
        }

        Ok(())
    }
}

#[async_trait]
impl SwitchAPI for FSN8550 {
    async fn get_ports(&self) -> Result<Vec<PortRepresentation>, SwitchApiError> {
        let ports = self
            .ports
            .get_or_try_init(|| async {
                debug!("try to init port list");
                let ports = DashSet::new();
                for port in self.retrieve_port_list().await? {
                    ports.insert(port);
                }
                Ok(ports)
            })
            .await?;
        Ok(ports
            .iter()
            .map(|i| PortRepresentation::new(i.key().clone()))
            .collect())
    }

    async fn add_untagged_port(
        &self,
        vlan_id: &VlanID,
        port_id: &PortID,
    ) -> Result<(), SwitchApiError> {
        self.alter_port(port_id, Some(vlan_id)).await
    }

    async fn remove_port(&self, port_id: &PortID) -> Result<(), SwitchApiError> {
        self.alter_port(port_id, None).await
    }

    async fn logout(&self) -> Result<(), SwitchApiError> {
        // nothing to do as the switch is operated via RESTCONF and basic auth.
        // There is no concept like a session cookie.
        Ok(())
    }

    async fn setup(&self, vlan_ids: Vec<VlanID>) -> Result<(), SwitchSetupError> {
        if !self.switch_details.critical_ports.mgmt_ports.is_empty() {
            error!("Management Ports are not allowed to be configured for now. Use the designated management interface.");
            return Err(SwitchSetupError::InternalError);
        }

        let _guard = self.setup_guard.acquire().await.map_err(|e| {
            warn!(error=?e, "could not get setup lock");
            SwitchSetupError::InternalError
        })?;

        // setup VLANs
        let url = self
            .build_url("vlans:vlans")
            .map_err(|_| SwitchSetupError::InternalError)?;

        let vlans: FSN8550VlansModule = FSN8550VlansModule {
            data: FSN8550VlansContainer {
                vlan_id: vlan_ids
                    .iter()
                    .map(|v| FSN8550VlansVlanIdContainer {
                        id: v.vlan_id(),
                        vlan_name: format!("vlan{}", v),
                        l3_interface: None,
                    })
                    .collect(),
            },
        };

        let res = self
            .client
            .put(url)
            .json(&vlans)
            .header(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static("application/yang-data+json"),
            )
            .send()
            .await
            .map_err(|e| {
                warn!(error = ?e, "could not sent request");
                SwitchSetupError::UnexpectedResponseFromSwitch
            })?;

        if !res.status().is_success() {
            let code = res.status();
            let body: serde_json::Value = res.json().await.unwrap_or_default();
            warn!(status = ?code, msg = ?body, "could not set up VLAN IDs");
            return Err(SwitchSetupError::UnexpectedResponseFromSwitch);
        }

        debug!("VLANs configured");

        // setup trunk ports
        let url = self
            .build_url("interface:interface")
            .map_err(|_| SwitchSetupError::InternalError)?;

        let build_interface = |p: &PortID| FSN8550InterfaceGigabitEthernetContainer {
            name: format!("xe-{}", p),
            disable: false,
            family: FSN8550InterfaceGigabitEthernetFamilyEthernetFamilyContainer {
                ethernet_switching:
                    FSN8550InterfaceGigabitEthernetFamilyEthernetSwitchingContainer {
                        native_vlan_id: self.switch_details.default_vlan.vlan_id(),
                        port_mode: PortMode::Trunk,
                        vlan: FSN8550InterfaceGigabitEthernetFamilyEthernetSwitchingVlanContainer {
                            members: vlan_ids
                                .iter()
                                .map(|v| VlanMember {
                                    id: v.vlan_id(),
                                    untagged: None,
                                })
                                .collect(),
                        },
                    },
            },
        };

        let interfaces = FSN8550InterfacesModule {
            data: FSN8550InterfacesNode {
                gigabit_ethernet: self
                    .switch_details
                    .critical_ports
                    .trunk_ports
                    .iter()
                    .map(build_interface)
                    .collect(),
            },
        };

        let res = self
            .client
            .patch(url)
            .json(&interfaces)
            .header(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static("application/yang-data+json"),
            )
            .send()
            .await
            .map_err(|_| SwitchSetupError::UnexpectedResponseFromSwitch)?;

        if !res.status().is_success() {
            let code = res.status();
            let body: serde_json::Value = res.json().await.unwrap_or_default();
            warn!(status = ?code, msg = ?body, "could not set up Trunk Ports");
            return Err(SwitchSetupError::UnexpectedResponseFromSwitch);
        }

        drop(_guard);
        Ok(())
    }
}
