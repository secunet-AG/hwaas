// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::aliases::{MachineName, MachineNameStr, MachineNetworkInterfaceStr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use super::TaggedMachineNetworkInterface;
pub use super::machine_interface_set::MachineInterfaceSet;

/// Represents a mapping between [`MachineNames`](crate::aliases::MachineName) and their [`NetworkInterfaces`](crate::aliases::MachineNetworkInterface)
/// participating in the same network.
#[derive(Serialize, Debug, PartialEq, Eq, Clone, Deserialize, JsonSchema)]
#[schemars(
    description = "A mapping between machine names and their network interfaces participating in the same network."
)]
pub struct NetworkSetup(pub HashMap<MachineName, MachineInterfaceSet>);

impl NetworkSetup {
    /// Returns `true` if the given `machine_name` has an `interface` that is
    /// part of the network setup.
    pub fn contains(
        &self,
        machine_name: &MachineNameStr,
        interface: &MachineNetworkInterfaceStr,
    ) -> bool {
        self.0
            .get(machine_name)
            .map(|interfaces| interfaces.0.contains(interface))
            .unwrap_or(false)
    }

    /// Insert an interface into the [`NetworkSetup`].
    pub fn insert(
        &mut self,
        TaggedMachineNetworkInterface {
            machine_name,
            interface,
        }: TaggedMachineNetworkInterface,
    ) {
        let _ = self
            .0
            .entry(machine_name)
            .or_insert_with(|| MachineInterfaceSet(HashSet::new()))
            .0
            .insert(interface);
    }

    /// Remove an interface from the [`NetworkSetup`].
    /// Returns `false` if the interface was not found
    /// in the [`NetworkSetup`].
    pub fn remove(
        &mut self,
        machine_name: &MachineNameStr,
        interface: &MachineNetworkInterfaceStr,
    ) -> bool {
        let (removed, also_remove_key) = self
            .0
            .get_mut(machine_name)
            .map(|interfaces| {
                let removed = interfaces.0.remove(interface);
                let also_remove_key = interfaces.0.is_empty();
                (removed, also_remove_key)
            })
            .unwrap_or((false, false));
        if also_remove_key {
            let _ = self.0.remove(machine_name);
        }
        removed
    }

    /// Returns an empty [`NetworkSetup`].
    pub fn empty() -> Self {
        Self(HashMap::new())
    }

    fn with_capacity(capacity: usize) -> Self {
        Self(HashMap::with_capacity(capacity))
    }
}

impl FromIterator<TaggedMachineNetworkInterface> for NetworkSetup {
    fn from_iter<T: IntoIterator<Item = TaggedMachineNetworkInterface>>(iter: T) -> Self {
        let iter = iter.into_iter();
        let (min_size, max_size) = iter.size_hint();
        let mut setup = Self::with_capacity(max_size.unwrap_or(min_size));
        for TaggedMachineNetworkInterface {
            machine_name,
            interface,
        } in iter
        {
            let _ = setup
                .0
                .entry(machine_name)
                .or_insert_with(|| MachineInterfaceSet(HashSet::new()))
                .0
                .insert(interface);
        }

        setup
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;
    use std::str::FromStr;

    use crate::network::test_fixtures;

    use super::*;

    /// Parses a JSON representation to a NetworkSetup.
    ///
    /// This function cuts corners and is only intended for test cases.
    ///
    /// # Motivation
    ///
    /// The actual `Deserialize` implementation of `NetworkSetup` is more
    /// sophisticated in the sense that it actually checks the format of
    /// the inputs and is also optimized for better performance. The point
    /// of this function is to have a very simple implementation that can be
    /// assumed to have correct behavior on trusted input which we can then
    /// test against.
    pub fn parse_setup(representation: &str) -> NetworkSetup {
        let representation: Value = Value::from_str(representation).unwrap();
        NetworkSetup(
            representation
                .as_object()
                .unwrap()
                .into_iter()
                .map(|(machine_name, interfaces)| {
                    (
                        machine_name.clone(),
                        MachineInterfaceSet(
                            interfaces.as_object().unwrap().keys().cloned().collect(),
                        ),
                    )
                })
                .collect(),
        )
    }

    /// Checks that the type serializes to JSON equivalent to `expected`.
    pub fn check_serialization<T: Serialize + std::fmt::Debug>(input: T, expected: &str) {
        let expected_json: Value = Value::from_str(expected).unwrap();

        let input_serialized_to_value: Value = serde_json::to_value(&input)
            .unwrap_or_else(|_| panic!("could not serialize {:#?} to JSON", dbg!(&input)));

        assert_eq!(input_serialized_to_value, expected_json);

        // Also check that serialization to string parsed to Value coincides with expected_json
        let input_serialized_to_string: String = serde_json::to_string(&input).unwrap();
        assert_eq!(
            Value::from_str(&input_serialized_to_string).unwrap(),
            expected_json
        );
    }

    /// Checks that the type deserializes to the expected NetworkSetup.
    pub fn check_deserialization(input: &str, expected: &NetworkSetup) {
        // Check deserialization from bytes
        let actual: NetworkSetup = serde_json::from_slice(input.as_bytes()).unwrap();
        assert_eq!(&actual, expected);

        // Now check deserialization from str
        let actual: NetworkSetup = serde_json::from_str(input).unwrap();
        assert_eq!(&actual, expected);

        // Now check deserialization from Value
        let value = serde_json::Value::from_str(input).unwrap();
        let actual: NetworkSetup = serde_json::from_value(value).unwrap();
        assert_eq!(&actual, expected);
    }

    // Naively parse valid JSON representations into a NetworkSetup
    // and check that it serializes to JSON equivalent of the starting point
    #[test]
    fn serialization() {
        for json_representation in test_fixtures::VALID_JSON_REPRESENTATIONS {
            let setup = parse_setup(json_representation);
            check_serialization(setup, json_representation);
        }
    }

    // Check that valid JSON representations deserialize to the same
    // NetworkSetup as produced by the naive parser.
    #[test]
    fn deserialization() {
        for json_representation in test_fixtures::VALID_JSON_REPRESENTATIONS {
            let expected = parse_setup(json_representation);
            check_deserialization(json_representation, &expected);
        }
    }

    // Check that deserialize followed by serialize provides JSON equivalent
    // to the starting point.
    #[test]
    fn deserialize_then_serialize() {
        for json_representation in test_fixtures::VALID_JSON_REPRESENTATIONS {
            let deserialization: NetworkSetup = serde_json::from_str(json_representation).unwrap();
            check_serialization(deserialization, json_representation);
        }
    }

    // Ensure that incorrectly formatted input fails
    // deserialization.
    #[test]
    fn deserialize_invalid() {
        let machine_only: &str = r#"{"machine1": {}}"#;
        let misformed_interface_set: &str = r#"
        {
            "machine1": {
                "LAN1": {
                    "foo" : "bar"
                }
            }
        }"#;

        let invalid_representations = [machine_only, misformed_interface_set];

        for json_representation in invalid_representations {
            let deserialization: Result<NetworkSetup, _> =
                serde_json::from_str(json_representation);
            assert!(deserialization.is_err());
        }
    }

    // Approval test to ensure that we notice when the generated
    // JSON schema for NetworkSetup changes.
    #[test]
    fn json_schema() {
        let expected_schema: &str = r##"
        {
            "$schema": "http://json-schema.org/draft-07/schema#",
            "title": "NetworkSetup",
            "description": "A mapping between machine names and their network interfaces participating in the same network.",
            "type": "object",
            "additionalProperties": {
              "$ref": "#/definitions/MachineInterfaceSet"
            },
            "definitions": {
              "EmptyMap": {
                "description": "An empty map.",
                "type": "object",
                "maxProperties": 0,
                "additionalProperties": false
              },
              "MachineInterfaceSet": {
                "description": "A collection of network interfaces.",
                "type": "object",
                "minProperties": 1,
                "additionalProperties": {
                  "$ref": "#/definitions/EmptyMap"
                }
              }
            }
          }"##;

        let expected_schema: Value = Value::from_str(expected_schema).unwrap();

        let actual_schema = schemars::schema_for!(NetworkSetup);
        let actual_schema: Value = serde_json::to_value(actual_schema).unwrap();

        assert_eq!(actual_schema, expected_schema);
    }
}
