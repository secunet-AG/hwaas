// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::collections::{HashMap, HashSet};

use serde::{de::Visitor, ser::SerializeMap, Deserialize, Serialize};

use schemars::{
    gen::SchemaGenerator,
    schema::{Schema, SchemaObject},
    JsonSchema,
};

use crate::aliases::MachineNetworkInterface;

/// A type representing a set of network interfaces.
///
/// # Serialization
/// This type has a custom implementation of `Serialize` and `Deserialize`
/// in order to represent sets in JSON as a map where values are empty
/// maps.
#[derive(Debug, PartialEq, Eq, Clone, JsonSchema)]
#[schemars(description = "A collection of network interfaces.")]
pub struct MachineInterfaceSet(
    #[schemars(schema_with = "interface_set_structure_schema")] pub HashSet<MachineNetworkInterface>,
);

// NOTE: This is tested in the tests for serializing a NetworkSetup.
impl Serialize for MachineInterfaceSet {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for interface in self.0.iter().map(String::as_str) {
            map.serialize_entry(interface, &EmptyMap(()))?;
        }
        map.end()
    }
}

// NOTE: This is tested in the tests for deserializing a NetworkSetup.
impl<'de> Deserialize<'de> for MachineInterfaceSet {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        type Map = serde_json::Map<String, serde_json::Value>;
        struct InterfaceSetVisitor;

        impl<'de> Visitor<'de> for InterfaceSetVisitor {
            type Value = MachineInterfaceSet;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(
                    formatter,
                    "a map where keys are network interfaces and values are empty maps"
                )
            }
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                // Loop over the map entries where keys should be strings representing network interfaces and values should be
                // empty maps. If our expectations are met we collect the keys into a set.
                let mut set: HashSet<MachineNetworkInterface> =
                    HashSet::with_capacity(map.size_hint().unwrap_or_default());
                while let Some((interface, value)) =
                    // Use Map and check that it is empty, rather than EmptyMap
                    // in order to provide better error messages.
                    map.next_entry::<MachineNetworkInterface, Map>()?
                {
                    let num_entries = value.len();
                    if num_entries == 0 {
                        set.insert(interface);
                    } else {
                        return Err(<A::Error as serde::de::Error>::invalid_length(
                            num_entries,
                            &"an empty map following the network interface",
                        ));
                    }
                }
                // Ensure that the set of network interfaces is non-empty.
                if !set.is_empty() {
                    Ok(MachineInterfaceSet(set))
                } else {
                    Err(<A::Error as serde::de::Error>::invalid_length(
                        0,
                        &"a non-empty map of interfaces",
                    ))
                }
            }
        }

        deserializer.deserialize_map(InterfaceSetVisitor {})
    }
}

#[derive(Debug)]
pub(super) struct MapNotEmptyError;
impl std::fmt::Display for MapNotEmptyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("map not empty")
    }
}

/// An empty map.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(
    try_from = "serde_json::Map<String, serde_json::Value>",
    into = "serde_json::Map<String, serde_json::Value>"
)]
#[derive(JsonSchema)]
pub(super) struct EmptyMap(#[schemars(schema_with = "empty_map_structure_schema")] pub(super) ());

impl TryFrom<serde_json::Map<String, serde_json::Value>> for EmptyMap {
    type Error = MapNotEmptyError;
    fn try_from(value: serde_json::Map<String, serde_json::Value>) -> Result<Self, Self::Error> {
        value
            .is_empty()
            .then_some(Self(()))
            .ok_or(MapNotEmptyError {})
    }
}

impl From<EmptyMap> for serde_json::Map<String, serde_json::Value> {
    fn from(_: EmptyMap) -> Self {
        serde_json::Map::new()
    }
}

// Updates the JSON schema for `EmptyMap` with structural requirements
fn empty_map_structure_schema(gen: &mut SchemaGenerator) -> Schema {
    type Map = serde_json::Map<String, serde_json::Value>;
    let mut schema: SchemaObject = Map::json_schema(gen).into();

    // Disallow additional properties and set max properties = 0.
    let object_validation = schema
        .object
        .as_deref_mut()
        .expect("schema object obtained from Map should have object set");
    object_validation.additional_properties = Some(Schema::Bool(false).into());
    object_validation.max_properties = Some(0);
    schema.into()
}

// Updates the JSON schema for `NetworkInterfaceSet` with structural requirements.
fn interface_set_structure_schema(gen: &mut SchemaGenerator) -> Schema {
    let mut schema: SchemaObject =
        HashMap::<MachineNetworkInterface, EmptyMap>::json_schema(gen).into();
    // We disallow empty network interface sets
    let object_validation = schema
        .object
        .as_deref_mut()
        .expect("schema object obtained from BTreeMap should have object set");
    object_validation.min_properties = Some(1);

    schema.into()
}
