// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

//! JSON data returned by Aruba Switch

use network_type_ids::VlanID;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ArubaVlanId {
    #[serde(flatten)]
    vlan_id: VlanID,

    #[serde(skip)]
    _is_management_vlan: bool,

    name: String,
}

impl ArubaVlanId {
    /// Aruba switches assign names to VLANs.
    /// Instead of letting the switch choose one, we are defining one here.
    /// All test networks are prefixed with `tst`
    fn name_from_id(id: u16) -> String {
        format!("tst{}", id)
    }
}

impl From<VlanID> for ArubaVlanId {
    fn from(vlan_id: VlanID) -> Self {
        Self {
            name: ArubaVlanId::name_from_id(vlan_id.vlan_id()),
            vlan_id,
            _is_management_vlan: false,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::aruba::aruba_vlan_id::ArubaVlanId;
    use network_type_ids::VlanID;
    use serde_json::json;

    #[test]
    fn test_expected_json_from_vlan_id() {
        let id = 5_u16;
        let struct_vlan_id = VlanID::new(id).expect("Valid VLAN ID");
        let aruba_vlan_id: ArubaVlanId = struct_vlan_id.into();
        let aruba_vlan_id_json = serde_json::to_value(aruba_vlan_id).expect("JSON");
        let expected_json = json!({
            "vlan_id": id,
            "name": ArubaVlanId::name_from_id(id)
        });

        assert_eq!(aruba_vlan_id_json, expected_json)
    }
}
