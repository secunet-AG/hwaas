// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;
use std::sync::RwLock;

use crate::network_type_ids::PortID;
use crate::network_type_ids::PortRepresentation;
use crate::network_type_ids::VlanID;
use crate::switch::SwitchApiError;
use async_trait::async_trait;
use dashmap::DashMap;

use crate::switch::api::SwitchAPI;
use crate::switch::switch_setup_error::SwitchSetupError;

#[derive(Clone)]
pub struct DummyTestSwitch {
    active: Arc<RwLock<bool>>,
    /// Map to simulate presence of ports. Keys = port IDs, Values = VLAN the port is a member of
    /// (assumption: it is only allowed to enable/disable Untagged ports, i.e. max. 1 owner VLAN),
    /// If a port is not activated, the Vlan is set to None.
    ports: DashMap<PortID, Option<VlanID>>,
    /// Biggest allowed PortID
    max_allowed_id: PortID,
}

impl DummyTestSwitch {
    pub fn new(num_ports: usize) -> Self {
        // Constructor, DummyTestSwitch has num_ports ports
        // Reason: make provided IDs in unit tests more understandable
        let ports: DashMap<PortID, Option<VlanID>> =
            DashMap::from_iter((0..num_ports).map(|i| (PortID::from(i + 1), None)));

        let max_allowed_id: PortID = num_ports.into();

        DummyTestSwitch {
            active: Arc::new(RwLock::new(true)),
            ports,
            max_allowed_id,
        }
    }
}

impl Default for DummyTestSwitch {
    fn default() -> Self {
        Self::new(48)
    }
}

#[async_trait]
impl SwitchAPI for DummyTestSwitch {
    async fn get_ports(&self) -> Result<Vec<PortRepresentation>, SwitchApiError> {
        Ok(self
            .ports
            .iter()
            .map(|i| PortRepresentation::new(i.key().clone()))
            .collect())
    }

    async fn add_untagged_port(
        &self,
        vlan_id: &VlanID,
        port_id: &PortID,
    ) -> Result<(), SwitchApiError> {
        if port_id > &self.max_allowed_id {
            Err(SwitchApiError::IDInvalid)
        } else {
            self.ports.insert(port_id.clone(), Some(vlan_id.clone()));
            Ok(())
        }
    }

    async fn remove_port(&self, port_id: &PortID) -> Result<(), SwitchApiError> {
        if port_id > &self.max_allowed_id {
            Err(SwitchApiError::IDInvalid)
        } else {
            self.ports.remove(port_id);
            Ok(())
        }
    }

    async fn logout(&self) -> Result<(), SwitchApiError> {
        *self.active.write().unwrap() = false;
        Ok(())
    }

    async fn setup(&self, _vlan_ids: Vec<VlanID>) -> Result<(), SwitchSetupError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::network_type_ids::PortID;
    use crate::network_type_ids::PortRepresentation;
    use crate::network_type_ids::VlanID;
    use crate::switch::DummyTestSwitch;
    use crate::switch::api::SwitchAPI;

    #[tokio::test]
    async fn test_get_ports() {
        // test if get_ports returns a list of Ports with expected config

        // build switch with all ports disabled, i.e. no Port belongs to a VLAN
        let num_ports = 10;
        let api = DummyTestSwitch::new(num_ports);
        // call get_ports
        let resp_vec: Vec<PortRepresentation> = api.get_ports().await.unwrap();
        // check if exactly the expected # of ports were returned
        assert_eq!(resp_vec.len(), num_ports);
        // check if returned ports all ports have the expected config
        for id in 1..num_ports + 1 {
            assert!(resp_vec.contains(&PortRepresentation::new(PortID::new(id.to_string()))))
        }
    }

    #[tokio::test]
    async fn test_add_untagged_port() {
        let port_id = PortID::new(9.to_string());
        let vlan_id = VlanID::new(42).unwrap();
        // test if add_untagged_port is working correctly, i.e. only the correct port is enabled
        // Assumption: test_get_ports succeeded
        let api = DummyTestSwitch::default();
        // add port
        api.add_untagged_port(&vlan_id, &port_id).await.unwrap();
        // get updated list of ports
        let resp_vec: Vec<PortRepresentation> = api.get_ports().await.unwrap();
        // check if correct port was enabled
        assert!(resp_vec.contains(&PortRepresentation::new(port_id)));
        // check if no other port were enabled
        assert_eq!(api.ports.iter().filter(|v| v.value().is_some()).count(), 1);
    }

    #[tokio::test]
    async fn test_remove_port() {
        // test if remove_port is working correctly, i.e. only the correct port is disabled
        // Assumption: test_get_ports and test_add_untagged_port succeeded
        let port_id = PortID::new(9.to_string());
        let vlan_id = VlanID::new(42).unwrap();
        let api = DummyTestSwitch::default();
        // add port to have something to remove
        api.add_untagged_port(&vlan_id, &port_id).await.unwrap();
        // remove port again
        api.remove_port(&port_id).await.unwrap();
        // get updated list of ports
        let resp_vec: Vec<PortRepresentation> = api.get_ports().await.unwrap();
        // check if correct port was disabled
        assert!(!resp_vec.contains(&PortRepresentation::new(port_id)));
        // check if no other port was disabled
        assert_eq!(api.ports.iter().filter(|v| v.value().is_some()).count(), 0);
    }

    #[tokio::test]
    async fn test_idempotent_add() {
        // test if add_untagged_port is idempotent
        // Assumption: test_get_ports and test_add_untagged_port succeeded
        let port_id = PortID::new(9.to_string());
        let vlan_id = VlanID::new(42).unwrap();
        let api = DummyTestSwitch::default();
        // add port
        api.add_untagged_port(&vlan_id, &port_id).await.unwrap();
        api.add_untagged_port(&vlan_id, &port_id).await.unwrap();
        assert_eq!(api.ports.iter().filter(|v| v.value().is_some()).count(), 1);
    }

    #[tokio::test]
    async fn test_idempotent_rm() {
        // test if remove_port is idempotent
        // Assumption: test_get_ports, test_add_untagged_port and test_remove_port succeeded
        let port_id = PortID::new(9.to_string());
        let api = DummyTestSwitch::default();
        // add port to have something to remove
        api.add_untagged_port(&VlanID::new(42).unwrap(), &port_id)
            .await
            .unwrap();
        // remove port again
        api.remove_port(&port_id).await.unwrap();
        api.remove_port(&port_id).await.unwrap();
        assert_eq!(api.ports.iter().filter(|v| v.value().is_some()).count(), 0);
    }
}
