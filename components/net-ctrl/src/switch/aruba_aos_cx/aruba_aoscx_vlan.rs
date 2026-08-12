// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::network_type_ids::{IDParseError, VlanID};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Deserialize, Serialize, Debug)]
pub(crate) struct ArubaAosCxVlan {
    /// The unique ID of the VLAN.
    pub(crate) id: u16,

    /// Some name for the VLAN.
    pub(crate) name: String,
}

impl ArubaAosCxVlan {
    /// Aruba AOS CX switches assign names to VLANs.
    /// Instead of letting the switch choose one, we are defining one here.
    /// All test networks are prefixed with `tst`
    fn name_from_id(id: u16) -> String {
        format!("tst{}", id)
    }
}

impl Display for ArubaAosCxVlan {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

impl From<VlanID> for ArubaAosCxVlan {
    fn from(vlan_id: VlanID) -> Self {
        Self {
            name: ArubaAosCxVlan::name_from_id(vlan_id.vlan_id()),
            id: vlan_id.vlan_id(),
        }
    }
}

impl TryInto<VlanID> for &ArubaAosCxVlan {
    type Error = IDParseError;

    fn try_into(self) -> Result<VlanID, Self::Error> {
        VlanID::new(self.id)
    }
}

#[cfg(test)]
mod test {
    use crate::network_type_ids::VlanID;
    use crate::switch::aruba_aos_cx::aruba_aoscx_vlan::ArubaAosCxVlan;
    use serde_json::json;

    #[test]
    fn test() {
        let expected = json!({
            "id": 43,
            "name": "tst43"
        });
        let item = ArubaAosCxVlan::from(VlanID::new(43).unwrap());

        assert_eq!(expected, serde_json::to_value(item).unwrap());
    }
}
