// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

//! This module contains types for JSON pointers
//! that are valid in the context of a [`NetworkSetup`](crate::network::NetworkSetup).
//!
//! JSON pointers are used in JSON patches to identify the values of interest within
//! a JSON document. See the [IETF specification](https://datatracker.ietf.org/doc/html/rfc6901/)
//! for more information about JSON pointers.
use crate::aliases::MachineName;
use schemars::gen::SchemaGenerator;
use schemars::schema::Schema;
use schemars::schema::SchemaObject;
use schemars::schema::StringValidation;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use crate::network::TaggedMachineNetworkInterface;

#[derive(Debug)]
pub(super) struct JsonPtrErr;
impl std::fmt::Display for JsonPtrErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid JSON pointer")
    }
}

/// JSON Pointer to a `MachineName` within a `NetworkSetup` (e.g. "/abmr").
#[derive(Serialize, Deserialize, Clone, JsonSchema)]
#[serde(try_from = "String", into = "String")]
pub(super) struct MachineJsonPtr(
    #[schemars(schema_with = "machine_json_ptr_structure_schema")] pub(super) MachineName,
);
impl TryFrom<String> for MachineJsonPtr {
    type Error = JsonPtrErr;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.strip_prefix('/')
            .map(|s| Self(s.to_owned()))
            .ok_or(JsonPtrErr {})
    }
}
impl From<MachineJsonPtr> for String {
    fn from(val: MachineJsonPtr) -> String {
        format!("/{}", val.0)
    }
}

// Updates the JSON schema for `MachineJsonPtr` with structural requirements
fn machine_json_ptr_structure_schema(gen: &mut SchemaGenerator) -> Schema {
    let mut schema: SchemaObject = String::json_schema(gen).into();
    // Require the string to at least have length 2. It also needs to match a regex
    // that requires the string to start with a forward slash followed by characters that
    // are different from forward slash.
    schema.string = Some(Box::new(StringValidation {
        max_length: None,
        min_length: Some(2),
        pattern: Some(r"^\/([^\/]+)".to_string()),
    }));
    schema.into()
}

/// JSON Pointer to a `TaggedMachineNetworkInterface` within a `NetworkSetup` (e.g. "/abmr/lan1").
#[derive(Serialize, Deserialize, Clone, JsonSchema)]
#[serde(try_from = "String", into = "String")]
pub(super) struct InterfaceJsonPtr(
    #[schemars(schema_with = "tagged_interface_json_ptr_structure_schema")]
    pub(super)  TaggedMachineNetworkInterface,
);
impl From<InterfaceJsonPtr> for String {
    fn from(val: InterfaceJsonPtr) -> String {
        format!("/{}/{}", &val.0.machine_name, &val.0.interface)
    }
}

impl TryFrom<String> for InterfaceJsonPtr {
    type Error = JsonPtrErr;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.strip_prefix('/')
            .and_then(|rest| {
                rest.find('/')
                    .map(|sep_idx| (&rest[..sep_idx], &rest[(sep_idx + 1)..]))
            })
            .ok_or(JsonPtrErr {})
            .map(|(machine_id, interface)| {
                InterfaceJsonPtr(TaggedMachineNetworkInterface {
                    machine_name: machine_id.to_owned(),
                    interface: interface.to_owned(),
                })
            })
    }
}

// Updates the JSON schema for `InterfaceJsonPtr` with structural requirements
fn tagged_interface_json_ptr_structure_schema(gen: &mut SchemaGenerator) -> Schema {
    let mut schema: SchemaObject = String::json_schema(gen).into();
    // Require the string to at least have length 4. It also needs to match a regex that
    // requires the string start with a forward slash followed by two segments of characters
    // separated by exactly one forward slash.
    schema.string = Some(Box::new(StringValidation {
        max_length: None,
        min_length: Some(4),
        pattern: Some(r"^\/([^\/]+)(\/)([^\/]+)".to_string()),
    }));
    schema.into()
}
