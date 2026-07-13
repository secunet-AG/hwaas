// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::ops::Range;

use network_type_ids::{IDParseError, VlanID};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A Vector of VlanIDs.
/// This is a helper struct to
/// generate a vector of [`VlanID`]s from
/// a range the user provides.
#[derive(Eq, PartialEq, Debug)]
pub struct VlanIDVec {
    /// Vector of valid VLAN IDs
    pub vlan_ids: Vec<VlanID>,
}

impl TryFrom<Range<u16>> for VlanIDVec {
    type Error = IDParseError;

    fn try_from(value: Range<u16>) -> Result<Self, Self::Error> {
        let vlan_ids = value
            .into_iter()
            .map(VlanID::new)
            .collect::<Result<Vec<VlanID>, IDParseError>>()?;

        Ok(VlanIDVec { vlan_ids })
    }
}

impl TryFrom<SetupData> for VlanIDVec {
    type Error = IDParseError;

    fn try_from(value: SetupData) -> Result<Self, Self::Error> {
        value.vlan_id_range.try_into()
    }
}

/// The user provided input for switch setup.
/// Contains a range of u16 representing the allowed VLAN IDs.
#[derive(JsonSchema, Serialize, Deserialize)]
pub struct SetupData {
    pub vlan_id_range: Range<u16>,
}

#[cfg(test)]
mod test {
    use network_type_ids::IDParseError;
    use serde_json::json;

    use super::{SetupData, VlanIDVec};

    #[test]
    fn test_valid_range() {
        let data = serde_json::from_value::<SetupData>(json!({
            "vlan_id_range": {
                "start": 10,
                "end": 20
            }
        }))
        .unwrap();

        let vec_result: Result<VlanIDVec, IDParseError> = data.try_into();
        assert!(vec_result.is_ok());
    }

    #[test]
    fn test_range_include_zero() {
        let data = serde_json::from_value::<SetupData>(json!({
            "vlan_id_range": {
                "start": 0,
                "end": 20
            }
        }))
        .unwrap();

        let vec_result: Result<VlanIDVec, IDParseError> = data.try_into();
        assert!(vec_result.is_err());
    }

    #[test]
    fn test_range_over_max_id() {
        let data = serde_json::from_value::<SetupData>(json!({
            "vlan_id_range": {
                "start": 5000,
                "end": 5004
            }
        }))
        .unwrap();

        let vec_result: Result<VlanIDVec, IDParseError> = data.try_into();
        assert!(vec_result.is_err());
    }

    #[test]
    fn test_range_include_negative_int() {
        let data_result = serde_json::from_value::<SetupData>(json!({
            "vlan_id_range": {
                "start": -10,
                "end": 20
            }
        }));

        assert!(data_result.is_err());
    }

    #[test]
    fn test_range_inverse_overlap() {
        let data_result = serde_json::from_value::<SetupData>(json!({
            "vlan_id_range": {
                "start": 11,
                "end": 1
            }
        }));

        assert_eq!(
            VlanIDVec::try_from(data_result.unwrap()).unwrap(),
            VlanIDVec { vlan_ids: vec![] }
        );
    }

    #[test]
    fn test_try_from_setup_data() {
        // Valid ranges
        assert!(VlanIDVec::try_from(SetupData {
            vlan_id_range: 2..6_u16,
        })
        .is_ok());
        assert!(VlanIDVec::try_from(2..6_u16).is_ok());

        // invalid ranges
        assert!(VlanIDVec::try_from(SetupData {
            vlan_id_range: 0..6_u16,
        })
        .is_err());
        assert!(VlanIDVec::try_from(0..6_u16).is_err());
    }
}
