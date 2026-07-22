// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::aruba_aos_cx::aruba_aoscx_port::{
    ArubaAosCxInterface, ArubaAosCxInterfaceAdminState, ArubaAosCxInterfaceUserConfig,
    ArubaAosCxInterfaceVlanConf, ArubaAosCxPortVlanMode,
};
use crate::aruba_aos_cx::aruba_aoscx_vlan::ArubaAosCxVlan;
use crate::{SwitchAPI, SwitchApiError, SwitchSetupError};
use async_trait::async_trait;
use dashmap::DashSet;
use network_type_ids::{PortID, PortRepresentation, SwitchDetails, VlanID};
use reqwest::{Response, StatusCode};
use reqwest_cookie_store::CookieStoreMutex;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware, RequestBuilder};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{OnceCell, RwLock, Semaphore};
use tracing::{debug, error, info, instrument, warn};
use url::Url;

// This is the version of the switch API to use. Older Versions are served as well.
// The maximum allowed API version correlates to the switch's firmware version.
const ARUBA_AOS_CX_API_VERSION: &str = "v10.13";

/// A client for configuring an Aruba AOS CX Switch
#[derive(Clone)]
pub struct ArubaAosCxClient {
    /// A reqwest client for establishing a session to the switch and sending it requests.
    client: ClientWithMiddleware,

    /// Track whether to call `login()`
    login_attempts: Arc<RwLock<usize>>,

    ports: OnceCell<DashSet<PortID>>,

    /// Switch details containing the address and login data for a switch.
    switch_details: SwitchDetails,

    /// Base URI for all REST API calls
    root_uri: Url,

    /// permit to set up the trunk links and prevent inconsistency during parallel setup request
    setup_guard: Arc<Semaphore>,
}

impl ArubaAosCxClient {
    pub fn new(switch_details: SwitchDetails) -> Result<Self, SwitchApiError> {
        let cookie_store = Arc::new(CookieStoreMutex::default());

        // prepare a client
        let client = reqwest::Client::builder()
            .no_proxy()
            .cookie_store(true)
            .cookie_provider(cookie_store.clone())
            // Aruba AOS CX switches serve the REST API only via HTTPS with self signed certs
            // We do not want to manage these certs for now.
            .danger_accept_invalid_certs(true)
            .build()
            .map_err(|_| {
                error!("build reqwest client failed");
                SwitchApiError::BuiltFaultyRequestToSwitch
            })?;
        let client = ClientBuilder::new(client).build();
        let root_uri = Url::parse(&format!(
            "https://{}:{}/rest/{ARUBA_AOS_CX_API_VERSION}/",
            switch_details.ip,
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
            login_attempts: Arc::new(RwLock::new(0)),
            ports: OnceCell::new(),
            setup_guard: Arc::new(Semaphore::new(1)),
        })
    }

    /// build url to reach dst with requests.
    /// The suffix describes uri of resources to access.
    #[instrument(skip(self), level = "trace")]
    fn build_url(&self, suffix: &str) -> Result<Url, SwitchApiError> {
        self.root_uri
            .join(suffix.to_string().as_str())
            .map_err(|e| {
                warn!(error = %e, "build URL failed");
                SwitchApiError::BuiltFaultyRequestToSwitch
            })
    }

    #[instrument(skip(self), level = "debug")]
    async fn login(&self) -> Result<(), SwitchApiError> {
        let Some(credentials) = &self.switch_details.credentials else {
            Err(SwitchApiError::Unauthorized)?
        };

        //build and send actual login request, return session cookie on success
        let url = self.build_url("login")?;

        //POST login as multipart form request
        let login_data = reqwest::multipart::Form::new()
            .text("username", credentials.username.clone())
            .text("password", credentials.password.clone());
        let response = self
            .client
            .post(url)
            .multipart(login_data)
            .header("Accept", "*/*")
            .send()
            .await
            .map_err(|e| {
                error!(error = %e, "could not send login request");
                SwitchApiError::DestinationUnreachable
            })?;

        if !response.status().is_success() {
            warn!(
                http_status = %response.status(),
                ip = %self.switch_details.ip,
                "switch returned error for login request"
            );
            return Err(SwitchApiError::Unauthorized);
        }

        info!("logged in");

        Ok(())
    }

    /// get all port identifiers from the switch
    #[instrument(skip(self))]
    async fn retrieve_port_list(&self) -> Result<DashSet<PortID>, SwitchApiError> {
        let url = self.build_url("system/interfaces?depth=1")?;
        let res_ports = self
            .authenticated_request(move |client| client.get(url))
            .await
            .map_err(|_| SwitchApiError::DestinationUnreachable)?;

        let set = DashSet::from_iter(
            res_ports
                .json::<Vec<ArubaAosCxInterface>>()
                .await
                .map_err(|_| SwitchApiError::UnexpectedResponseFromSwitch)?
                .into_iter()
                .map(|v| PortID::from(v.port_id)),
        );
        debug!(len = set.len(), "port-list constructed");
        Ok(set)
    }

    /// Retries login and the HTTP request on HTTP 401 Unauthorized
    #[instrument(skip(self, f), fields(login_attempts, tries))]
    async fn authenticated_request<F>(&self, f: F) -> Result<Response, SwitchApiError>
    where
        F: Clone + FnOnce(&ClientWithMiddleware) -> RequestBuilder,
    {
        const MAX_TRIES: usize = 3;
        let mut tries = 0;
        loop {
            let login_attempt = *self.login_attempts.read().await;
            let req = (f.clone())(&self.client);
            tries += 1;
            //tracing::Span::current().record("tries", &tries);
            match req.send().await {
                // Request succeed and is ok
                Ok(res) if res.status().is_success() => return Ok(res),

                // Request is unauthorized and we exceeded the maximum number of login attempts
                Ok(res) if res.status() == StatusCode::UNAUTHORIZED && tries >= MAX_TRIES => {
                    error!("too many retries for UNAUTHORIZED request");
                    return Err(SwitchApiError::Unauthorized);
                }

                // Unauthorized, so we try to log in and resend the request in the next loop iteration
                Ok(res) if res.status() == StatusCode::UNAUTHORIZED => {
                    // relogin and retry in the next loop iteration.
                    // But first, synchronize and check if login() happened concurrently.
                    let mut login_attempts = self.login_attempts.write().await;
                    if *login_attempts == login_attempt {
                        // No other login attempt
                        *login_attempts += 1;
                        debug!(attempt = *login_attempts, "try login");
                        self.login().await?;
                    } else {
                        // Another login attempt happened,
                        // retry with the new session cookie.
                        debug!("another login attempt happened");
                    }
                }

                Ok(res) if res.status() == StatusCode::FORBIDDEN => {
                    error!(
                        http_status = %res.status(),
                        "no access"
                    );
                    return Err(SwitchApiError::UnexpectedResponseFromSwitch);
                }
                Ok(res) => {
                    warn!(
                        http_status = %res.status(),
                        "HTTP error for authenticated request"
                    );
                    return Ok(res);
                }
                Err(e) => {
                    error!(error = %e, "error sending original request");
                    return Err(SwitchApiError::UnexpectedResponseFromSwitch);
                }
            }
        }
    }

    async fn add_vlan_id(&self, vlan_id: &VlanID) -> Result<(), SwitchSetupError> {
        let url = self
            .build_url("system/vlans")
            .map_err(|_| SwitchSetupError::DestinationUnreachable)?;

        let vlan: ArubaAosCxVlan = vlan_id.clone().into();
        let response = self
            .authenticated_request(|client| client.post(url).json(&vlan))
            .await
            .map_err(|e| {
                warn!(error = %e, "could not get response");
                SwitchSetupError::UnexpectedResponseFromSwitch
            })?;

        match handle_response(response).await {
            Ok(_) => Ok(()),
            Err((StatusCode::INTERNAL_SERVER_ERROR, Value::String(v)))
                if v.contains("Internal service error") =>
            {
                error!("switch returned internal error - likely the VLAN exists already");
                Err(SwitchSetupError::VlanIdSetupError(vlan_id.clone()))
            }
            Err((c, v)) => {
                error!("switch returned {:?}: {:?}", c, v);
                Err(SwitchSetupError::VlanIdSetupError(vlan_id.clone()))
            }
        }
    }

    #[instrument(skip(self), level = "debug")]
    async fn set_interface_conf(
        &self,
        port_id: &PortID,
        conf: ArubaAosCxInterfaceVlanConf,
    ) -> Result<(), SwitchApiError> {
        let url = self.build_url(&format!("system/interfaces/{}", port_id.escaped()))?;

        let response = self
            .authenticated_request(|client| client.put(url).json(&conf))
            .await
            .map_err(|e| {
                warn!(error = %e, "could not get response");
                SwitchApiError::DestinationUnreachable
            })?;

        handle_response(response).await.map_err(|(_, v)| {
            error!(error = %v, "could not set port vlan conf");
            SwitchApiError::UnexpectedResponseFromSwitch
        })
    }

    async fn get_vlans(&self) -> Result<Vec<ArubaAosCxVlan>, SwitchApiError> {
        let url = self.build_url("system/vlans")?;
        let response = self
            .authenticated_request(|client| {
                client
                    .get(url)
                    .query(&[("attributes", "id,name"), ("depth", "2")])
            })
            .await?;

        if !response.status().is_success() {
            warn!(status_code = ?response.status(), "could not get VLANs form switch");
            return Err(SwitchApiError::UnexpectedResponseFromSwitch);
        };

        let res: HashMap<String, ArubaAosCxVlan> = response.json().await.map_err(|e| {
            warn!(error = ?e, "could not serialize VLAN list");
            SwitchApiError::UnexpectedResponseFromSwitch
        })?;

        Ok(res.into_values().collect())
    }

    fn make_vlan_uri(vlan_id: u16) -> String {
        format!("/rest/{ARUBA_AOS_CX_API_VERSION}/system/vlans/{}", vlan_id)
    }
}

/// Represent switch response as [`Result`].
/// On error return the code and the JSON message sent by the switch.
pub(crate) async fn handle_response(response: Response) -> Result<(), (StatusCode, Value)> {
    let response_status = response.status();
    let body = response.text().await.unwrap_or_default();
    let response_json = serde_json::from_str(body.as_str()).unwrap_or(Value::String(body));

    match response_status {
        code if code.is_success() => Ok(()),
        code if code.is_informational() => Ok(()),
        code => Err((code, response_json)),
    }
}

#[async_trait]
impl SwitchAPI for ArubaAosCxClient {
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
        return Ok(ports
            .iter()
            .map(|i| PortRepresentation::new(i.key().clone()))
            .collect());
    }

    async fn add_untagged_port(
        &self,
        vlan_id: &VlanID,
        port_id: &PortID,
    ) -> Result<(), SwitchApiError> {
        let vlan_uri = ArubaAosCxClient::make_vlan_uri(vlan_id.vlan_id());
        let trunk_vlan_conf = ArubaAosCxInterfaceVlanConf {
            vlan_trunks: vec![vlan_uri.clone()],
            vlan_mode: Some(ArubaAosCxPortVlanMode::Access),
            vlan_tag: Some(vlan_uri),
            user_config: Some(ArubaAosCxInterfaceUserConfig {
                admin: ArubaAosCxInterfaceAdminState::Up,
            }),
        };
        self.set_interface_conf(port_id, trunk_vlan_conf).await?;

        Ok(())
    }

    async fn remove_port(&self, port_id: &PortID) -> Result<(), SwitchApiError> {
        let vlan_uri = ArubaAosCxClient::make_vlan_uri(self.switch_details.default_vlan.vlan_id());
        let trunk_vlan_conf = ArubaAosCxInterfaceVlanConf {
            vlan_trunks: vec![],
            vlan_mode: Some(ArubaAosCxPortVlanMode::Access),
            vlan_tag: Some(vlan_uri),
            user_config: Some(ArubaAosCxInterfaceUserConfig {
                admin: ArubaAosCxInterfaceAdminState::Down,
            }),
        };
        self.set_interface_conf(port_id, trunk_vlan_conf).await?;

        Ok(())
    }

    async fn logout(&self) -> Result<(), SwitchApiError> {
        let url = self.build_url("logout")?;
        let response = self.client.post(url).send().await;

        match response {
            Ok(resp) if resp.status() == StatusCode::UNAUTHORIZED => {
                debug!("session is already gone");
                Ok(())
            }
            Ok(resp) if resp.status() == StatusCode::FORBIDDEN => {
                // we do not expect this case
                // maybe it could occur if the CSRF token is invalid
                error!("logout FORBIDDEN");
                Err(SwitchApiError::UnexpectedResponseFromSwitch)
            }
            Ok(resp) if resp.status().is_success() => {
                info!("logout successful");
                Ok(())
            }
            Ok(resp) => {
                let status_code = resp.status();
                let body = resp.text().await.unwrap_or_default();
                warn!(?status_code, body, "logout failed");
                Err(SwitchApiError::UnexpectedResponseFromSwitch)
            }
            Err(e) => {
                warn!(error = ?e, "sending logout command failed");
                Err(SwitchApiError::DestinationUnreachable)
            }
        }
    }

    async fn setup(&self, vlan_ids: Vec<VlanID>) -> Result<(), SwitchSetupError> {
        // get list of already allocated VLANs
        let available_vlans: Vec<u16> = self
            .get_vlans()
            .await
            .map_err(|e| {
                error!(?e, "could not get VLANs");
                SwitchSetupError::UnexpectedResponseFromSwitch
            })?
            .iter()
            .map(|i| i.id)
            .collect();

        // add new VLAN IDs
        // this part is not critical as long as VLANs are only added.
        // The implementation queries all VLANs of the switch within the critical path.
        for id in vlan_ids
            .iter()
            .filter(|i| !available_vlans.contains(&i.vlan_id()))
        {
            self.add_vlan_id(id).await?;
        }
        debug!(amount_added = vlan_ids.len(), "added VLAN IDs");

        // management port stays untouched for now - set them up is done manually
        debug!(
            "Ignored {} management ports",
            self.switch_details.critical_ports.mgmt_ports.len()
        );

        // set up all trunk ports
        // we need to know all VLANs to configure: New ones passed to the setup fn as well as the
        // existing ones. We could combine the two lists here but let's simply ask the Switch for it.
        // ATTENTION: this is safe at the moment because we only add VLANs to a switch but never delete.
        // a parallel setup call can alter the state as well!
        // It is critical if a parallel setup request is performed in between of getting the current
        // VLANs and setting up the trunks. So we use the setup_guard here for mitigation.
        let _guard = self.setup_guard.acquire().await.map_err(|e| {
            warn!(error=?e, "could not get setup lock");
            SwitchSetupError::InternalError
        })?;
        let all_vlans = self.get_vlans().await.map_err(|e| {
            error!(?e, "could not get VLANs");
            SwitchSetupError::UnexpectedResponseFromSwitch
        })?;
        // prepare the trunk vlan config
        let allowed_vlans: Vec<String> = all_vlans
            .iter()
            // forbid default traffic on trunks
            .filter(|v| v.id != self.switch_details.default_vlan.vlan_id())
            // forbid mgmt traffic on trunks
            .filter(|v| v.id != self.switch_details.mgmt_vlan.vlan_id())
            .map(|v| ArubaAosCxClient::make_vlan_uri(v.id))
            .collect::<Vec<String>>();

        // now apply it to all trunk ports
        for port_id in &self.switch_details.critical_ports.trunk_ports {
            // bring the port up
            // apply all allowed vlans
            // We do not set a vlan tag or mode, as we only want to allow tagged traffic to pass
            let trunk_vlan_conf = ArubaAosCxInterfaceVlanConf {
                vlan_trunks: allowed_vlans.clone(),
                vlan_mode: Some(ArubaAosCxPortVlanMode::NativeTagged),
                user_config: Some(ArubaAosCxInterfaceUserConfig {
                    admin: ArubaAosCxInterfaceAdminState::Up,
                }),
                ..Default::default()
            };
            self.set_interface_conf(port_id, trunk_vlan_conf)
                .await
                .map_err(|e| {
                    error!(
                        error = %e,
                        ?port_id,
                        "could not set VLAN config on trunk-port "
                    );
                    SwitchSetupError::CriticalPortSetupError(port_id.clone())
                })?;
        }

        drop(_guard);

        debug!(
            "Setup {} Trunk Ports",
            self.switch_details.critical_ports.trunk_ports.len()
        );

        Ok(())
    }
}
