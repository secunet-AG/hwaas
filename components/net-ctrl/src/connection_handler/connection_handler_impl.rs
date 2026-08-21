// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::sync::Arc;

use tracing::{debug, error, info, warn};

use crate::inventory::{InventoryConnector, SwitchMapping, SwitchModelDetail};
use crate::network_type_ids::{PortID, SwitchID};
use crate::switch::{SwitchAPI, SwitchApiError, SwitchBackend};

use super::connection_handler_error::ConnectionHandlerError;

/// This struct is responsible for caching individual SwitchAPI
/// sessions for a certain amount of time (TTL).
/// Obtaining a [`SwitchAPI`] is allowed by providing [`crate::inventory::SwitchModelDetail`].
/// The cache respects construction of a new switch session and minimizes
/// multiple constructions for the same [`crate::inventory::SwitchModelDetail`].
pub struct ConnectionHandler {
    sessions: HashMap<SwitchID, Arc<SwitchBackend>>,

    /// Inventory is needed to translate [`SwitchID`]s
    inventory: InventoryConnector,
}

impl ConnectionHandler {
    /// Inventory is needed to translate [`SwitchID`]s
    pub async fn new(inventory: InventoryConnector) -> Result<Self, ConnectionHandlerError> {
        let sessions = inventory
            .get_switch_mapping()
            .await?
            .into_iter()
            .map(|(switch_id, SwitchModelDetail { model, details })| {
                model.construct(details).map(|b| (switch_id, Arc::new(b)))
            })
            .collect::<Result<HashMap<SwitchID, Arc<SwitchBackend>>, SwitchApiError>>()?;

        Ok(ConnectionHandler {
            sessions,
            inventory,
        })
    }

    /// Get a [`crate::switch::SwitchBackend`] from the cache by specifying a [`SwitchID`].
    /// If it was never requested before or the TTL is over construct a new switch session.
    #[tracing::instrument(skip(self, switch_id), level = "debug")]
    pub fn get_switch_api(
        &self,
        switch_id: &SwitchID,
    ) -> Result<Arc<impl SwitchAPI>, ConnectionHandlerError> {
        if let Some(session) = self.sessions.get(switch_id) {
            Ok(session.clone())
        } else {
            Err(ConnectionHandlerError::SwitchNotFound)
        }
    }

    /// Query the inventory to get the [`SwitchMapping`].
    ///
    /// ## Returns
    /// * Ok value with [`SwitchMapping`]
    /// * ConnectionHandlerError::SwitchNotFound(e) on any inventory error
    pub async fn get_switches(&self) -> Result<SwitchMapping, ConnectionHandlerError> {
        Ok(self.inventory.get_switch_mapping().await?)
    }

    /// Compares a given [`PortID`] if it is contained within the list of critical ports of a Switch (referenced by [`SwitchID`])
    ///
    /// ## Returns
    /// A Result with Ok value as boolean if the port was a critical one or an error else.
    #[tracing::instrument(skip(self))]
    pub async fn is_port_forbidden(
        &self,
        switch_id: &SwitchID,
        port_id: &PortID,
    ) -> Result<bool, ConnectionHandlerError> {
        // test for preset forbidden port ids
        let critical_ports = self
            .inventory
            .get_switch_model_details(switch_id)
            .await
            .map_err(|e| {
                warn!(error = %e, "Could not get switch details");
                e
            })?
            .details
            .get_critical_ports()
            .clone();
        Ok(critical_ports
            .mgmt_ports
            .iter()
            .chain(critical_ports.trunk_ports.iter())
            .any(|id| id == port_id))
    }
}

impl Drop for ConnectionHandler {
    fn drop(&mut self) {
        let sessions = std::mem::take(&mut self.sessions);
        if sessions.is_empty() {
            // Nothing to do in case of just generating the OpenAPI spec.
            info!("no switch sessions to log out");
            return;
        }

        tokio::spawn(async move {
            for (sid, session) in sessions.into_iter() {
                debug!(?sid, "going to terminate");
                let _ = session
                    .logout()
                    .await
                    .map_err(|e| error!(error = %e, "Logout at switch failed"));
                debug!(?sid, "terminated");
            }
        });
        info!("all switch sessions terminated");
    }
}

#[cfg(test)]
mod test {
    use super::ConnectionHandler;
    use crate::inventory::{
        InventoryBackend, InventoryConnector, InventoryDummyBackend, SwitchMapping,
        SwitchModelDetail,
    };
    use crate::network_type_ids::{
        Credentials, CriticalPorts, PortID, SwitchDetails, SwitchID, VlanID,
    };
    use crate::switch::SwitchModel;
    use std::{
        net::{IpAddr, Ipv4Addr},
        sync::Arc,
    };

    static SWITCH: &str = "sw1";

    async fn get_conn_handler() -> ConnectionHandler {
        let mapping = SwitchMapping::from([(
            SwitchID::new(SWITCH.to_string()),
            SwitchModelDetail {
                model: SwitchModel::Dummy,
                details: SwitchDetails::new(
                    IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                    Some(Credentials {
                        username: "manager".to_string(),
                        password: "123".to_string(),
                    }),
                    CriticalPorts {
                        mgmt_ports: vec![PortID::new("1".to_string())],
                        trunk_ports: vec![PortID::new("2".to_string())],
                    },
                    VlanID::new(1).unwrap(),
                    VlanID::new(2).unwrap(),
                ),
            },
        )])
        .into();
        let inventory_backend: InventoryBackend = InventoryDummyBackend::new(mapping).into();
        let inventory = InventoryConnector::new(inventory_backend);
        ConnectionHandler::new(inventory).await.unwrap()
    }

    #[tokio::test]
    async fn test_get_switches() {
        let conn = get_conn_handler().await;
        let sid = SwitchID::new(SWITCH.to_string());
        assert!(conn.get_switches().await.unwrap().contains_key(&sid))
    }

    #[tokio::test]
    async fn test_get() {
        let conn = get_conn_handler().await;
        let sid = SwitchID::new(SWITCH.to_string());
        let sd1 = conn.get_switch_api(&sid).unwrap();
        let sd2 = conn.get_switch_api(&sid).unwrap();

        // check that they're the same in memory
        assert!(Arc::ptr_eq(&sd1, &sd2));
    }
}
