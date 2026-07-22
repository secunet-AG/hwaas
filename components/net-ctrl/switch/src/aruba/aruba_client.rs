// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::aruba::aruba_port_element::{ArubaPort, ArubaPortElementList};
use crate::aruba::aruba_vlan_id::ArubaVlanId;
use crate::aruba::aruba_vlan_ports::{ArubaVlanPort, ArubaVlanPortElementList, ArubaVlanPortMode};
use crate::aruba::unwrapped_response::UnwrappedResponse;
use crate::switch_api_errors::SwitchApiError;
use crate::switch_setup_error::SwitchSetupError;
use crate::SwitchAPI;
use async_trait::async_trait;
use dashmap::DashSet;
use network_type_ids::{PortID, PortRepresentation, SwitchDetails, VlanID};
use reqwest::StatusCode;
use reqwest_cookie_store::CookieStoreMutex;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware, RequestBuilder};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{OnceCell, RwLock};
use tracing::{debug, error, info, instrument, warn};
use url::Url;

/// A client for configuring an Aruba Switch
#[derive(Clone)]
pub struct ArubaClient {
    /// A reqwest client for establishing a session to the switch and sending it requests.
    client: ClientWithMiddleware,

    cookie_store: Arc<CookieStoreMutex>,

    /// Track whether to call `login()`
    login_attempts: Arc<RwLock<usize>>,

    ports: OnceCell<DashSet<PortID>>,

    /// Switch details containing the address and login data for a switch.
    switch_details: SwitchDetails,

    /// Base URI for all REST API calls
    root_uri: Url,
}

impl ArubaClient {
    /// Establishes a session with the switch by performing a login and returns an [`ArubaClient`].
    ///
    /// # Arguments
    ///
    /// * `switch_details` - Struct containing the switches management IPv4 Address and Login.
    ///
    #[instrument(skip(switch_details), level = "debug", fields(selector = format!("{:?}",switch_details.ip)))]
    pub fn new(switch_details: SwitchDetails) -> Result<Self, SwitchApiError> {
        //create client used for all subsequent requests to a given destination, contains session cookie
        //create jar needed to store a clients cookies
        let cookie_store = CookieStoreMutex::default();
        let cookie_store = Arc::new(cookie_store);

        // prepare a client
        let client = reqwest::Client::builder()
            .no_proxy()
            .cookie_provider(Arc::clone(&cookie_store))
            .build()
            .map_err(|_| {
                error!("build reqwest client failed");
                SwitchApiError::BuiltFaultyRequestToSwitch
            })?;
        let client = ClientBuilder::new(client).build();

        let root_uri = Url::parse(&format!(
            "http://{}:{}/rest/v1/",
            switch_details.ip,
            switch_details.port.unwrap_or(80)
        ))
        .map_err(|e| {
            warn!(error = %e, "Building base URL failed");
            SwitchApiError::DestinationUnreachable
        })?;

        Ok(ArubaClient {
            client,
            cookie_store,
            switch_details,
            root_uri,
            login_attempts: Arc::new(RwLock::new(0)),
            ports: OnceCell::new(),
        })
    }

    /// build url to reach dst with requests.
    /// The suffix describes uri of resources to access.
    #[instrument(skip(self), level = "trace")]
    fn build_url(&self, suffix: &str) -> Result<Url, SwitchApiError> {
        self.root_uri.join(suffix).map_err(|e| {
            warn!(error = %e, "Build URL failed");
            SwitchApiError::BuiltFaultyRequestToSwitch
        })
    }

    #[instrument(skip(self), level = "debug")]
    async fn login(&self) -> Result<(), SwitchApiError> {
        let Some(credentials) = &self.switch_details.credentials else {
            Err(SwitchApiError::Unauthorized)?
        };
        //build and send actual login request, return session cookie on success
        let url = self.build_url("login-sessions")?;
        let body = json!({
            "userName": credentials.username.clone(),
            "password": credentials.password.clone()
        });
        //POST login request
        let response = self
            .client
            .post(url)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                error!(error = %e, "Could not send login request");
                SwitchApiError::DestinationUnreachable
            })?;

        //extract json body and return entry "cookie"
        let cookie_value = match response {
            r if r.status().is_success() || r.status().is_informational() => r
                .json::<Value>()
                .await
                .map_err(|e| {
                    error!(
                        error = %e,
                        ip = %self.switch_details.ip,
                        "Could not parse login response"
                    );
                    SwitchApiError::UnexpectedResponseFromSwitch
                })?
                .get("cookie")
                .cloned()
                .ok_or_else(|| {
                    // implicit assumption about the switch behavior:
                    // if code was a 200, the response always contains a cookie
                    warn!(ip = %self.switch_details.ip, "Response JSON does not contain the key 'cookie'");
                    SwitchApiError::Unauthorized
                })?,
            r if r.status() == StatusCode::SERVICE_UNAVAILABLE => {
                warn!("likely the aximum amount of sessions was reached");
                Err(SwitchApiError::UnexpectedResponseFromSwitch)?
            },
            r => {
                warn!(
                    http_status = %r.status(),
                    ip = %self.switch_details.ip,
                    "Switch returned error response"
                );
                Err(SwitchApiError::UnexpectedResponseFromSwitch)?
            }
        };

        let cookie = match cookie_value {
            Value::String(cookie) if !cookie.is_empty() => {
                info!(ip = %self.switch_details.ip, "Got session cookie.");
                cookie
            }
            _ => {
                error!(ip = %self.switch_details.ip, "Switch returned no cookie string");
                Err(SwitchApiError::UnexpectedResponseFromSwitch)?
            }
        };

        // lock the cookie store for insertion
        let mut cs = self.cookie_store.lock().map_err(|e| {
            warn!(error = %e, "Could not lock cookie store");
            SwitchApiError::Unauthorized
        })?;

        // try parse the cookie string and insert it
        cs.parse(cookie.as_str(), &self.root_uri).map_err(|e| {
            warn!(error = %e, ip = %self.switch_details.ip, "Could not parse cookie");
            SwitchApiError::Unauthorized
        })?;
        // lock guard cs goes out of scope now
        drop(cs);

        info!("logged in");
        Ok(())
    }

    /// Retries login and the HTTP request on HTTP 401 Unauthorized
    async fn authenticated_request<F>(&self, f: F) -> Result<UnwrappedResponse, SwitchApiError>
    where
        F: Clone + FnOnce(&ClientWithMiddleware) -> RequestBuilder,
    {
        const MAX_TRIES: usize = 3;
        let mut tries = 0;
        loop {
            let login_attempt = *self.login_attempts.read().await;
            let req = (f.clone())(&self.client);
            tries += 1;
            match req.send().await {
                Ok(res) if res.status().is_success() => {
                    return Ok(UnwrappedResponse {
                        status_code: res.status(),
                        body: res.text().await.unwrap_or_default(),
                    });
                }
                Ok(res) => {
                    let status = res.status();
                    let body = res.text().await.unwrap_or_default();

                    warn!(?status, body, "original request was not successful");

                    if (status == StatusCode::BAD_REQUEST
                        && (body.contains("Please login")
                            || body.contains("Access is unauthorized")))
                        || status == StatusCode::UNAUTHORIZED
                    {
                        if tries >= MAX_TRIES {
                            return Err(SwitchApiError::Unauthorized);
                        }
                        // Else, relogin and retry in the next loop iteration.
                        // But first, synchronize and check if login() happened concurrently.
                        let mut login_attempts = self.login_attempts.write().await;
                        if *login_attempts == login_attempt {
                            // No other login attempt
                            *login_attempts += 1;
                            self.login().await?;
                        } else {
                            // Another login attempt happened,
                            // retry with the new session cookie.
                            debug!("another login attempt was made")
                        }
                    } else {
                        // the original request failed, and we do not identify it as login issue
                        warn!(
                            http_status = %status,
                            http_body = ?body,
                            "HTTP error on authenticated request"
                        );
                        return Ok(UnwrappedResponse {
                            status_code: status,
                            body,
                        });
                    }
                }
                Err(e) => {
                    error!(error = %e, "Error logging into switch");
                    return Err(SwitchApiError::UnexpectedResponseFromSwitch);
                }
            }
        }
    }

    #[instrument(skip(self), level = "debug")]
    async fn retrieve_port_list(&self) -> Result<DashSet<PortID>, SwitchApiError> {
        let url = self.build_url("ports")?;
        let res_ports = self
            .authenticated_request(move |client| client.get(url))
            .await
            .map_err(|_| SwitchApiError::DestinationUnreachable)?;
        Ok(DashSet::from_iter(
            serde_json::from_str::<ArubaPortElementList>(res_ports.body.as_str())
                .map_err(|_| SwitchApiError::UnexpectedResponseFromSwitch)?
                .port_element
                .into_iter()
                .map(|v: ArubaPort| PortID::from(v.id)),
        ))
    }

    /// Get the first VLAN the Port is a member of.
    ///
    /// Untagged ports only have one VLAN assigned.
    #[instrument(skip(self), level = "debug")]
    async fn get_first_vlan_id_of_port(&self, port_id: &PortID) -> Result<VlanID, SwitchApiError> {
        let url = self.build_url("vlans-ports")?;

        // Request list of all VLAN member ports
        let res = self
            .authenticated_request(|client| client.get(url))
            .await
            .map_err(|e| {
                warn!(error = %e, "could not send request to obtain vlan list");
                SwitchApiError::DestinationUnreachable
            })?;

        // Filter out VLAN that contains the port
        let vlan_ports = serde_json::from_str::<ArubaVlanPortElementList>(res.body.as_str())
            .map_err(|e| {
                warn!(error = %e, "Response was not expected");
                SwitchApiError::UnexpectedResponseFromSwitch
            })?;

        vlan_ports
            .vlan_port_element
            .iter()
            .find(|w| w.port_id == port_id.to_string())
            .map(|v| v.vlan_id.clone())
            .ok_or_else(|| {
                warn!(
                    "Could not find PortID in vlan_ports: {:#?}",
                    vlan_ports.vlan_port_element
                );
                SwitchApiError::IDInvalid
            })
    }

    #[instrument(skip(self), level = "debug")]
    async fn set_port_status(&self, port_id: &PortID, status: bool) -> Result<(), SwitchApiError> {
        let url = self.build_url(&format!("ports/{}", port_id))?;
        let m = HashMap::from([("is_port_enabled", status)]);

        let response = self
            .authenticated_request(|client| client.put(url).json(&m))
            .await
            .map_err(|e| {
                warn!(error = %e, "Could not get response");
                SwitchApiError::DestinationUnreachable
            })?;

        handle_response_verbose(response).await.map_err(|(_, v)| {
            warn!(error = %v, "Could not set port status");
            SwitchApiError::IDInvalid
        })
    }

    #[instrument(skip(self), level = "debug")]
    async fn remove_non_default_vlans(
        &self,
        port_id: &PortID,
        vlan_id: &VlanID,
    ) -> Result<(), SwitchApiError> {
        let url = self.build_url(&format!("vlans-ports/{}-{}", vlan_id, port_id))?;

        match vlan_id == &self.switch_details.default_vlan {
            true => {
                info!("Skip delete VLAN: keep default VLAN for Port {}", port_id);
                Ok(())
            }
            false => {
                let response = self
                    .authenticated_request(|client| client.delete(url))
                    .await
                    .map_err(|e| {
                        warn!(error = %e, "Could not get response");
                        SwitchApiError::DestinationUnreachable
                    })?;

                handle_response_verbose(response).await.map_err(|_| {
                    warn!("Could not remove");
                    SwitchApiError::IDInvalid
                })
            }
        }
    }

    async fn add_port_to_vlan(&self, vlan_port: &ArubaVlanPort) -> Result<(), SwitchApiError> {
        let url = self.build_url("vlans-ports")?;
        let response = self
            .authenticated_request(|client| client.post(url).json(&vlan_port))
            .await
            .map_err(|e| {
                warn!(error = %e, "Could not get response");
                SwitchApiError::UnexpectedResponseFromSwitch
            })?;

        match handle_response(response).await {
            Ok(()) => Ok(()),
            Err((StatusCode::BAD_REQUEST, v)) if v == json!({"message": "Association exists"}) => {
                // In this case the port is already associated to the VLAN.
                // Attention: The association is not updated in this case!
                debug!("Association exists - no changes made");
                Ok(())
            }
            Err(_) => Err(SwitchApiError::UnexpectedResponseFromSwitch),
        }
    }

    #[instrument(skip(self))]
    async fn add_vlan_id(&self, vlan_id: &VlanID) -> Result<(), SwitchSetupError> {
        let url = self
            .build_url("vlans")
            .map_err(|_| SwitchSetupError::DestinationUnreachable)?;

        let m: ArubaVlanId = vlan_id.clone().into();
        let response = self
            .authenticated_request(|client| client.post(url).json(&m))
            .await
            .map_err(|e| {
                warn!(error = %e, "Could not get response");
                SwitchSetupError::UnexpectedResponseFromSwitch
            })?;

        match handle_response(response).await {
            Ok(_) => Ok(()),
            Err((StatusCode::BAD_REQUEST, v)) if v == json!({"message": "VLAN exists"}) => Ok(()),
            Err((c, v)) => {
                warn!("Switch returned {:?}: {:?}", c, v);
                Err(SwitchSetupError::VlanIdSetupError(vlan_id.clone()))
            }
        }
    }

    /// Takes a vector of VLAN IDs and removes the VLAN IDs
    /// of the mgmt and the default VLAN.
    fn filtered_vlans(&self, vlan_ids: &[VlanID]) -> Vec<VlanID> {
        vlan_ids
            .iter()
            .filter(|e| {
                e.ne(&&self.switch_details.default_vlan) && e.ne(&&self.switch_details.mgmt_vlan)
            })
            .cloned()
            .collect()
    }
}

#[async_trait]
impl SwitchAPI for ArubaClient {
    async fn get_ports(&self) -> Result<Vec<PortRepresentation>, SwitchApiError> {
        let ports = self
            .ports
            .get_or_try_init(|| async {
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

    /// Enable a port and add it to a VLAN (untagged).
    /// If there was already an untagged VLAN
    /// assigned, the assignment will be replaced.
    async fn add_untagged_port(
        &self,
        vlan_id: &VlanID,
        port_id: &PortID,
    ) -> Result<(), SwitchApiError> {
        if vlan_id == &self.switch_details.default_vlan {
            warn!("could not enable port within the default VLAN");
            return Err(SwitchApiError::IDInvalid);
        }

        // currently the switch sends a 400 "Association exists"
        // if the port is already assigned to the VLAN.
        // Therefore, we first check if the desired state matches the
        // present state.
        match self.get_first_vlan_id_of_port(port_id).await.ok() {
            Some(vlan) if vlan == *vlan_id => debug!("Port is already assigned to this VLAN"),
            _ => {
                // add port to vlan as Untagged
                let vlan_port: ArubaVlanPort = (port_id, vlan_id).into();
                self.add_port_to_vlan(&vlan_port).await?;
            }
        };

        // enable port
        self.set_port_status(port_id, true).await?;
        Ok(())
    }

    /// Disable port and remove it from the VLAN it was a member of
    async fn remove_port(&self, port_id: &PortID) -> Result<(), SwitchApiError> {
        // Disable port
        self.set_port_status(port_id, false).await?;

        // Get VLAN that port is a member of
        // There is actually only one VLAN to delete for "user-accessible" ports.
        // A port could only have one untagged VLAN assigned.
        // Deleting the first one equals to deleting the only one.
        let vlan_id = self.get_first_vlan_id_of_port(port_id).await?;

        // Remove Port from VLAN
        self.remove_non_default_vlans(port_id, &vlan_id).await?;

        Ok(())
    }

    async fn logout(&self) -> Result<(), SwitchApiError> {
        // Dropping ALL sessions of the Aruba switch receiving the request.
        info!(ip = %self.switch_details.ip, "Logout at Switch.");
        let url = self.build_url("login-sessions")?;
        let response = self
            .client
            .delete(url)
            .send()
            .await
            .map_err(|_| SwitchApiError::DestinationUnreachable)?;

        let response = UnwrappedResponse {
            status_code: response.status(),
            body: response.text().await.unwrap_or_default(),
        };

        handle_response_verbose(response)
            .await
            .map_err(|_| SwitchApiError::UnexpectedResponseFromSwitch)?;

        Ok(())
    }

    async fn setup(&self, vlan_ids: Vec<VlanID>) -> Result<(), SwitchSetupError> {
        // add all VLAN IDs
        for id in &vlan_ids {
            self.add_vlan_id(id).await?;
        }
        debug!("Added {} VLAN IDs", vlan_ids.len());

        // management port stays untouched for now - set them up is done manually
        debug!(
            "Ignored {} management ports",
            self.switch_details.critical_ports.mgmt_ports.len()
        );

        // set up all trunk ports
        for port_id in &self.switch_details.critical_ports.trunk_ports {
            self.set_port_status(port_id, true)
                .await
                .map_err(|_| SwitchSetupError::CriticalPortSetupError(port_id.clone()))?;

            // in the following forbid mode correspond to arubas GVRP VLAN
            // learning. See: https://en.wikipedia.org/wiki/Multiple_Registration_Protocol
            // forbid GVRP for management VLANs on trunks ports.
            let mgmt_vlan_port = ArubaVlanPort {
                vlan_id: self.switch_details.mgmt_vlan.clone(),
                port_id: port_id.to_string(),
                port_mode: ArubaVlanPortMode::Forbidden,
            };
            self.add_port_to_vlan(&mgmt_vlan_port).await.map_err(|e| {
                error!(
                error = %e,
                        "Could not set forbid mode for management VLAN {} on trunk-port {}",
                        self.switch_details.default_vlan, port_id,
                    );
                SwitchSetupError::CriticalPortSetupError(port_id.clone())
            })?;

            // forbid default traffic on trunks
            let default_vlan_port = ArubaVlanPort {
                vlan_id: self.switch_details.default_vlan.clone(),
                ..mgmt_vlan_port
            };
            self.add_port_to_vlan(&default_vlan_port)
                .await
                .map_err(|e| {
                    error!(
                    error = %e,
                                "Could not set forbid mode for default VLAN {} on trunk-port {}",
                                self.switch_details.default_vlan, port_id,
                            );
                    SwitchSetupError::CriticalPortSetupError(port_id.clone())
                })?;

            // add non-default and non-mgmt as tagged VLAN.
            for vlan_id in self.filtered_vlans(&vlan_ids) {
                let vlan_port = ArubaVlanPort {
                    vlan_id: vlan_id.clone(),
                    port_id: port_id.to_string(),
                    port_mode: ArubaVlanPortMode::Tagged,
                };
                self.add_port_to_vlan(&vlan_port).await.map_err(|e| {
                    error!(
                    error = %e,
                                "Could not setup VLAN {} on trunk-port {}",
                                vlan_id, port_id,
                            );
                    SwitchSetupError::TrunkTaggedVlanSetupError(port_id.clone(), vlan_id.clone())
                })?;
            }
        }
        debug!(
            "Setup {} Trunk Ports",
            self.switch_details.critical_ports.trunk_ports.len()
        );

        Ok(())
    }
}

/// The same as [`handle_response`] but additionally warn on error responses.
async fn handle_response_verbose(response: UnwrappedResponse) -> Result<(), (StatusCode, Value)> {
    handle_response(response)
        .await
        .map_err(|(code, response_json)| {
            warn!("Switch returned {:?}: {:?}", code, response_json);
            (code, response_json)
        })
}

/// Represent switch response as [`Result`].
/// On error return the code and the JSON message sent by the switch.
async fn handle_response(response: UnwrappedResponse) -> Result<(), (StatusCode, Value)> {
    let response_json =
        serde_json::from_str(response.body.as_str()).unwrap_or(Value::String(response.body));

    match response.status_code {
        code if code.is_success() => Ok(()),
        code if code.is_informational() => Ok(()),
        code => Err((code, response_json)),
    }
}

#[cfg(test)]
mod test {
    use crate::aruba::aruba_client::ArubaClient;
    use network_type_ids::{CriticalPorts, SwitchDetails, VlanID};
    use reqwest_cookie_store::CookieStoreMutex;
    use reqwest_middleware::ClientBuilder;
    use std::net::{IpAddr, Ipv4Addr};
    use std::str::FromStr;
    use std::sync::Arc;
    use tokio::sync::{OnceCell, RwLock};
    use url::Url;

    #[test]
    fn test_filter_vlans() {
        let default_vlan = VlanID::new(1).unwrap();
        let mgmt_vlan = VlanID::new(2).unwrap();

        let sd = SwitchDetails::new(
            IpAddr::V4(Ipv4Addr::from_str("1.1.1.1").unwrap()),
            None,
            CriticalPorts {
                mgmt_ports: vec![],
                trunk_ports: vec![],
            },
            default_vlan.clone(),
            mgmt_vlan.clone(),
        );

        let ac = ArubaClient {
            switch_details: sd,
            client: ClientBuilder::new(reqwest::Client::new()).build(),
            cookie_store: Arc::new(CookieStoreMutex::default()),
            root_uri: Url::parse("http://127.0.0.1/api/v1").unwrap(),
            login_attempts: Arc::new(RwLock::new(0)),
            ports: OnceCell::new(),
        };

        let unfiltered_vlan_ids: Vec<VlanID> = (1..6).map(|e| VlanID::new(e).unwrap()).collect();

        let filtered_vlan_ids = ac.filtered_vlans(&unfiltered_vlan_ids);

        assert_eq!(unfiltered_vlan_ids.len(), 5);

        println!("filtered {:?}", filtered_vlan_ids);

        assert_ne!(filtered_vlan_ids, unfiltered_vlan_ids);

        assert_eq!(filtered_vlan_ids.len(), unfiltered_vlan_ids.len() - 2);
        assert_eq!(
            filtered_vlan_ids.iter().find(|e| (*e).eq(&default_vlan)),
            None
        );
        assert_eq!(filtered_vlan_ids.iter().find(|e| (*e).eq(&mgmt_vlan)), None);
    }
}
