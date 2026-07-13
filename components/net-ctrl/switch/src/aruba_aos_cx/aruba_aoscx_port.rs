// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use network_type_ids::VlanID;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub(crate) struct ArubaAosCxInterface {
    /// The unique ID of the VLAN.
    #[serde(flatten)]
    pub(crate) vlan_id: VlanID,

    /// The unique ID of the Port.
    pub(crate) port_id: String,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ArubaAosCxInterfaceAdminState {
    Up,
    Down,
}

impl From<bool> for ArubaAosCxInterfaceAdminState {
    fn from(value: bool) -> Self {
        if value {
            ArubaAosCxInterfaceAdminState::Up
        } else {
            ArubaAosCxInterfaceAdminState::Down
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub(crate) struct ArubaAosCxInterfaceUserConfig {
    /// The user-configured administrative state of Interface.
    pub(crate) admin: ArubaAosCxInterfaceAdminState,
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub(crate) struct ArubaAosCxInterfaceVlanConf {
    /// VLAN mode for ports with 'routing' being 'false'. For those ports, it has to be set,
    /// otherwise the port will be held down. When vlan_mode is not set, it indicates that the port
    /// is in access mode.
    pub(crate) vlan_mode: Option<ArubaAosCxPortVlanMode>,

    /// Specifies the Access or Native VLAN for this port.
    pub(crate) vlan_tag: Option<String>,

    /// List of VLANs that this port is allowed to pass traffic for. When the list is empty,
    /// it means that the port will be allowed to pass traffic for all VLANs configured on the device.
    /// This is only relevant if 'vlan_mode' is 'native-tagged' or 'native-untagged' and ignored
    /// for 'access'. 'native-tagged' or 'native-untagged' port always trunks its native ('vlan_tag')
    /// VLAN, regardless of whether it's included in 'vlan_trunks'.
    pub(crate) vlan_trunks: Vec<String>,

    /// Key-value pairs that stores the user configuration of Interface.
    pub(crate) user_config: Option<ArubaAosCxInterfaceUserConfig>,
}

#[derive(Deserialize, Serialize, Debug, Default)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ArubaAosCxPortVlanMode {
    /// Port can carry traffic for only one VLAN and the VLAN is specified as part of vlan_tag.
    /// Packets ingressing and egressing this port will not have an 802.1Q VLAN tag. When the port
    /// is trunked, mode must be either native-tagged or native-untagged, value contained in
    /// vlan_trunks refers to the list of VLANs which have to be trunked, if it is empty then all
    /// VLANs have to be trunked.
    #[default]
    Access,

    /// Port can carry traffic for multiple VLANs. One of the VLANs is designated as native and is
    /// specified as part of vlan_tag. Traffic for all VLANs on this port including the native VLAN
    /// will be 802.1Q VLAN tagged.
    NativeTagged,

    /// Port can carry traffic for multiple VLANs. One of the VLANs is designated as native and the
    /// VLAN ID is specified as part of vlan_tag. Traffic for all VLANs except the native VLAN will
    /// be 802.1Q VLAN tagged Traffic for the native VLAN will not have an 802.1Q tag.
    NativeUntagged,
}
