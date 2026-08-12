// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::network_type_ids::{PortID, PortRepresentation, SwitchDetails, VlanID};
use crate::switch::aruba::aruba_client::ArubaClient;
use crate::switch::fs_n8550::fs_picos::FSN8550;
use crate::switch::switch_api_errors::SwitchApiError;
use crate::switch::{
    SwitchSetupError, aruba_aos_cx::aruba_aoscx_client::ArubaAosCxClient,
    dummy::dummy_test_switch::DummyTestSwitch,
};
use async_trait::async_trait;
use enum_dispatch::enum_dispatch;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// Client API to configure Switches.
///
/// Offers generic functionality to configure switch models of different vendors over their
/// Rest API.
#[async_trait]
#[enum_dispatch]
pub trait SwitchAPI {
    /// Request list of all ports of a switch. Fails when request is invalid or switch is unreachable.
    ///
    /// On success returns a data structure containing information about a port
    async fn get_ports(&self) -> Result<Vec<PortRepresentation>, SwitchApiError>;

    /// Add a port to a VLAN in Untagged mode. Implicitly enables the port. Fails when receiving
    /// an invalid id, port is already enabled or when trying to access a forbidden VLAN/port, i.e. Mgmt VLAN, its ports or
    /// ports connecting to the switching topology.
    ///
    /// # Arguments
    ///
    /// * `vlan_id` - ID of the VLAN that will receive a new member port.
    /// * `port_id` - Target port to become a member of a VLAN.
    async fn add_untagged_port(
        &self,
        vlan_id: &VlanID,
        port_id: &PortID,
    ) -> Result<(), SwitchApiError>;

    /// Disable the port given by port_id. Implicitly remove it from the VLAN it belonged
    /// to prior. Fails when port is already disabled, port id is invalid or access to the port
    /// is forbidden.
    ///
    ///  # Arguments
    ///
    /// * `port_id` - Target port of the operation. Needs to be a string.
    async fn remove_port(&self, port_id: &PortID) -> Result<(), SwitchApiError>;

    /// Disconnect the current session if there is one.
    ///
    ///  # Returns
    ///
    /// * OK(()) - on successful logout or not needed for the backend
    /// * [`SwitchApiError`] - if there was a logout attempt that failed.
    async fn logout(&self) -> Result<(), SwitchApiError>;

    /// Add all VLAN IDs and configure critical ports
    /// all non-critical ports stay untouched and
    /// no VLAN ID is deleted
    ///
    /// ## Arguments
    ///
    /// * `switch_details` is needed to determine the default VLAN and critical ports
    /// * `vlan_ids` contain all VLAN IDs to set up
    ///
    /// ## Returns
    ///
    /// * Ok(()) on success
    /// * [`SwitchApiError`] if there was some unrecoverable error during setup
    async fn setup(&self, vlan_ids: Vec<VlanID>) -> Result<(), SwitchSetupError>;
}

#[enum_dispatch(SwitchAPI)]
#[derive(Clone)]
pub enum SwitchBackend {
    ArubaClient,
    ArubaAosCxClient,
    DummyTestSwitch,
    FSN8550,
    //other APIs
}

#[derive(Debug, Copy, JsonSchema, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum SwitchModel {
    Aruba,
    Aruba2540,
    ArubaAosCx,
    ArubaCx6100,
    FsPicos,
    Dummy,
    Dummy24,
    Dummy48,
}

impl Display for SwitchModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_string(self).unwrap_or("?".to_string())
        )
    }
}

impl SwitchModel {
    pub fn construct(
        &self,
        switch_details: SwitchDetails,
    ) -> Result<SwitchBackend, SwitchApiError> {
        match self {
            SwitchModel::Aruba | SwitchModel::Aruba2540 => {
                ArubaClient::new(switch_details).map(|api| api.into())
            }
            SwitchModel::ArubaAosCx | SwitchModel::ArubaCx6100 => {
                ArubaAosCxClient::new(switch_details).map(|api| api.into())
            }
            SwitchModel::FsPicos => FSN8550::new(switch_details).map(|api| api.into()),
            SwitchModel::Dummy => Ok(DummyTestSwitch::default().into()),
            SwitchModel::Dummy48 => Ok(DummyTestSwitch::new(48).into()),
            SwitchModel::Dummy24 => Ok(DummyTestSwitch::new(24).into()),
        }
    }
}
