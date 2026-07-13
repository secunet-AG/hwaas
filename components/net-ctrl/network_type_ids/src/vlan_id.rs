// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use core::fmt;
use schemars::JsonSchema;
use serde::de::{Unexpected, Visitor};
use serde::{de, Deserialize, Deserializer, Serialize};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::ops::Range;

#[derive(Debug)]
pub enum IDParseError {
    InvalidVlanID,
}

impl Error for IDParseError {}

impl Display for IDParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let msg = match self {
            IDParseError::InvalidVlanID => "Invalid VLAN ID",
        };

        f.write_str(msg)
    }
}

/// Wrapper type for different VLAN IDs potentially used for different use cases.
#[derive(
    Serialize, Deserialize, JsonSchema, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash,
)]
pub struct VlanID {
    /// Standard 802.1q ID.
    #[serde(deserialize_with = "VlanID::deserialize_validated")]
    vlan_id: u16,
}

impl VlanID {
    pub fn vlan_id(&self) -> u16 {
        self.vlan_id
    }

    pub fn new(id: u16) -> Result<Self, IDParseError> {
        VlanID::validate(id).map(|vlan_id| VlanID { vlan_id })
    }

    const fn range() -> Range<u16> {
        1..4094
    }

    fn validate(id: u16) -> Result<u16, IDParseError> {
        if VlanID::range().contains(&id) {
            Ok(id)
        } else {
            Err(IDParseError::InvalidVlanID)
        }
    }

    fn deserialize_validated<'de, D>(e: D) -> Result<u16, D::Error>
    where
        D: Deserializer<'de>,
    {
        e.deserialize_u16(VlanIDDeserializeValidatedVisitor)
    }
}

struct VlanIDDeserializeValidatedVisitor;
impl<'de> Visitor<'de> for VlanIDDeserializeValidatedVisitor {
    type Value = u16;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "a u16 within {:?}", VlanID::range())
    }

    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_u16(v as u16)
    }

    fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        VlanID::validate(v)
            .map_err(|_| de::Error::invalid_value(Unexpected::Unsigned(v as u64), &self))
    }

    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_u16(v as u16)
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_u16(v as u16)
    }
}

impl Display for VlanID {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.vlan_id().to_string())
    }
}

#[cfg(test)]
mod tests {
    use crate::VlanID;
    use serde_json::json;

    #[test]
    fn vlan_id_incorrect_type() {
        let res = serde_json::from_value::<VlanID>(json!({"vlan_id": "string"}));
        assert!(res.is_err());
    }

    #[test]
    fn vlan_id_out_of_bounds() {
        let res = serde_json::from_value::<VlanID>(json!({"vlan_id": 65535}));
        assert!(res.is_err());
    }

    #[test]
    fn vlan_id_correct_deserialize() {
        let res = serde_json::from_value::<VlanID>(json!({"vlan_id": 42}));
        assert!(res.is_ok());
    }
}
