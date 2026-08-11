// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

mod json_ptr;
mod serialization_repr;

use std::ops::ControlFlow;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::aliases::{MachineName, MachineNameStr, MachineNetworkInterfaceStr};

use super::{MachineInterfaceSet, NetworkSetup, TaggedMachineNetworkInterface};

use serialization_repr::{AddOpInner, RemoveOpInner};

/// A JSON Patch (See the [IETF specification](https://datatracker.ietf.org/doc/html/rfc6902/)) that
/// may be applied to a `NetworkSetup`.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct NetworkSetupPatch(pub Vec<NetworkSetupPatchOp>);

impl NetworkSetupPatch {
    /// Check whether this patch can be applied to the given `network_setup`.
    ///
    /// The error variant wraps the patch operation causing the problem.
    ///
    /// ## Additional user defined control flow
    ///
    /// The provided `interface_inspector` is called for each traversed network interface that is associated with an `add` operation.
    /// If the `interface_inspector` returns the `Break` variant then an error wrapping the current `NetworkSetupPatchOp` is returned.
    ///
    /// ## Performance characteristics
    ///
    /// This method does not allocate at all, but traverses up to `<number of patch operations in self>^2/2` patch operations
    /// in the worst case. As patches should be small we expect this trade off to be favorable in most situations.
    pub fn is_applicable<F>(
        &self,
        network_setup: &NetworkSetup,
        mut interface_inspector: F,
    ) -> Result<(), &NetworkSetupPatchOp>
    where
        F: FnMut(&MachineNameStr, &MachineNetworkInterfaceStr) -> ControlFlow<()>,
    {
        // Helper function to check if a machine_name entry should exist in the network setup after applying the given `applied_patch_ops`
        let machine_exists =
            |machine_name: &MachineNameStr, applied_patch_ops: &[NetworkSetupPatchOp]| -> bool {
                // First check what happened last of the two: Add op with the given machine => OK, Remove op with the given machine => Not OK.
                // If neither of the two occurred, then check if there is a `machine_name` entry in the original `network_setup`
                let final_add_or_remove_machine = applied_patch_ops
                    .iter()
                    .rev()
                    .find(|op| op.affects_machine(machine_name));
                if let Some(last_add_or_remove_machine) = final_add_or_remove_machine {
                    matches!(last_add_or_remove_machine, NetworkSetupPatchOp::Add(..))
                } else {
                    network_setup.0.contains_key(machine_name)
                }
            };

        // Helper function to reduce duplication
        fn user_approves_interface<'patch_op, F>(
            machine_name: &MachineNameStr,
            interface: &MachineNetworkInterfaceStr,
            op: &'patch_op NetworkSetupPatchOp,
            f: &mut F,
        ) -> Result<(), &'patch_op NetworkSetupPatchOp>
        where
            F: FnMut(&MachineNameStr, &MachineNetworkInterfaceStr) -> ControlFlow<()>,
        {
            if let ControlFlow::Break(_) = f(machine_name, interface) {
                Err(op)
            } else {
                Ok(())
            }
        }

        for (idx, op) in self.0.iter().enumerate() {
            match op {
                NetworkSetupPatchOp::Add(add_op) => {
                    match add_op {
                        AddOp::MachineWithInterfaces {
                            machine,
                            interfaces,
                        } => {
                            // This kind of op is always permitted by the specification. Call the user defined function
                            // to check if the user permits it as well
                            for interface in interfaces.0.iter() {
                                user_approves_interface(
                                    machine,
                                    interface,
                                    op,
                                    &mut interface_inspector,
                                )?;
                            }
                        }
                        AddOp::Interface(tagged_interface) => {
                            if machine_exists(&tagged_interface.machine_name, &self.0[..idx]) {
                                // The machine the interface belogns to has an entry in the network setup so the interface may be added.
                                // Call the user defined function to check that the user permits this addition as well.
                                user_approves_interface(
                                    &tagged_interface.machine_name,
                                    &tagged_interface.interface,
                                    op,
                                    &mut interface_inspector,
                                )?;
                            } else {
                                return Err(op);
                            }
                        }
                    }
                }
                NetworkSetupPatchOp::Remove(remove_op) => {
                    match remove_op {
                        RemoveOp::Machine(machine_name) => {
                            if !machine_exists(machine_name, &self.0[..idx]) {
                                // Cannot remove a machine that should not be part of the network setup
                                // at this point.
                                return Err(op);
                            }
                        }
                        RemoveOp::TaggedInterface(tagged_interface) => {
                            // Make sure that the interface should exist in the network setup after applying
                            // all previous patch operations
                            if let Some(interface_added_or_removed_op) =
                                &self.0[..idx].iter().rev().find(|prev_op| {
                                    prev_op.affects_interface(
                                        &tagged_interface.machine_name,
                                        &tagged_interface.interface,
                                    )
                                })
                            {
                                if matches!(
                                    interface_added_or_removed_op,
                                    NetworkSetupPatchOp::Remove(..)
                                ) {
                                    // The last patch affecting `tagged_interface` was a removal
                                    // hence trying to remove it again is an error
                                    return Err(op);
                                }
                            } else {
                                // The interface was neither added nor removed by any of the previous operations so we check that it
                                // exists in the original network setup
                                if !network_setup.contains(
                                    &tagged_interface.machine_name,
                                    &tagged_interface.interface,
                                ) {
                                    return Err(op);
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

/// Parsed JSON Patch operation.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum NetworkSetupPatchOp {
    Remove(#[schemars(with = "RemoveOpInner")] RemoveOp),
    Add(#[schemars(with = "AddOpInner")] AddOp),
}

impl NetworkSetupPatchOp {
    /// Check whether this operation attempts to adds or delete the given `interface` of the machine with
    /// id `machine_name` from a network setup.
    fn affects_interface(
        &self,
        machine_name: &MachineNameStr,
        interface: &MachineNetworkInterfaceStr,
    ) -> bool {
        match self {
            Self::Add(add_op) => match add_op {
                AddOp::Interface(tagged_interface) => {
                    tagged_interface.machine_name == machine_name
                        && tagged_interface.interface == interface
                }
                AddOp::MachineWithInterfaces {
                    machine,
                    interfaces,
                } => {
                    machine == machine_name
                        && interfaces
                            .0
                            .iter()
                            .any(|current_interface| current_interface == interface)
                }
            },
            Self::Remove(remove_op) => match remove_op {
                RemoveOp::Machine(machine) => machine == machine_name,
                RemoveOp::TaggedInterface(tagged_interface) => {
                    tagged_interface.machine_name == machine_name
                        && tagged_interface.interface == interface
                }
            },
        }
    }

    /// Checks whether this operation attempts to add or remove the given machine with id `machine_name`.
    fn affects_machine(&self, machine_name: &MachineNameStr) -> bool {
        match self {
            Self::Add(add_op) => {
                if let AddOp::MachineWithInterfaces { machine, .. } = add_op {
                    machine == machine_name
                } else {
                    false
                }
            }
            Self::Remove(remove_op) => {
                if let RemoveOp::Machine(machine) = remove_op {
                    machine == machine_name
                } else {
                    false
                }
            }
        }
    }
}

/// Parsed remove operation from a JSON Patch.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(try_from = "RemoveOpInner", into = "RemoveOpInner")]
pub enum RemoveOp {
    /// Remove the given [`TaggedMachineNetworkInterface`]
    /// from the `NetworkSetup`.
    TaggedInterface(TaggedMachineNetworkInterface),
    /// Remove all network interfaces of the given [`MachineName`]
    /// from the `NetworkSetup`.
    Machine(MachineName),
}

/// Parsed add operation from a JSON Patch.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(try_from = "AddOpInner", into = "AddOpInner")]
pub enum AddOp {
    /// Add the given [`TaggedMachineNetworkInterface`] to the `NetworkSetup`.
    Interface(TaggedMachineNetworkInterface),
    /// Add a machine with the given [`MachineName`]
    /// and the network interfaces in [`MachineInterfaceSet`] to the `NetworkSetup`.
    ///
    /// ## Behavior when the machine is already present
    ///
    /// If interfaces from the given machine are already present in the `NetworkSetup`
    /// then they will be replaced by the interfaces provided in this patch as per the
    /// JSON Patch specification.
    MachineWithInterfaces {
        machine: MachineName,
        interfaces: MachineInterfaceSet,
    },
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use schemars::schema_for;
    use serde_json::{Value, json};

    #[test]
    fn deserialize_patch() {
        let patch =
            serde_json::Value::from_str(std::include_str!("test_fixtures/patch.json")).unwrap();
        let deserialized_patch: NetworkSetupPatch = serde_json::from_value(patch.clone()).unwrap();
        let roundtrip_patch: Value = serde_json::to_value(&deserialized_patch).unwrap();
        assert_eq!(roundtrip_patch, patch);
    }

    #[test]
    fn json_schema() {
        let expected: Value = serde_json::Value::from_str(std::include_str!(
            "test_fixtures/expected_patch_json_schema.json"
        ))
        .unwrap();
        let actual_schema = schema_for!(NetworkSetupPatch);
        let actual: Value = serde_json::to_value(&actual_schema).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn is_applicable_add_machine() {
        let patch: NetworkSetupPatch = serde_json::from_value(json!(
            [
                {"op": "add", "path": "/abmr1", "value": {"lan1": {}}}
            ]
        ))
        .unwrap();

        let network_setup = NetworkSetup::empty();

        assert!(
            patch
                .is_applicable(&network_setup, |_, _| ControlFlow::Continue(()))
                .is_ok()
        );
    }

    #[test]
    fn is_applicable_add_machine_then_interface() {
        let patch: NetworkSetupPatch = serde_json::from_value(json!(
            [
                {"op": "add", "path": "/abmr1", "value": {"lan1": {}}},
                {"op": "add", "path": "/abmr1/lan2", "value": {}}
            ]
        ))
        .unwrap();

        let network_setup = NetworkSetup::empty();

        assert!(
            patch
                .is_applicable(&network_setup, |_, _| ControlFlow::Continue(()))
                .is_ok()
        );
    }

    #[test]
    fn is_applicable_add_interface() {
        let patch: NetworkSetupPatch = serde_json::from_value(json!(
            [
                {"op": "add", "path": "/abmr1/lan2", "value": {}}
            ]
        ))
        .unwrap();

        let network_setup: NetworkSetup = serde_json::from_value(json!(
            {
                "abmr1": {"lan1": {}}
            }
        ))
        .unwrap();

        let interface_inspector =
            |_: &MachineNameStr, _: &MachineNetworkInterfaceStr| ControlFlow::Continue(());

        assert!(
            patch
                .is_applicable(&network_setup, interface_inspector)
                .is_ok()
        );

        // Also check that the patch is not applicable if there is no entry for /abmr1
        assert!(
            patch
                .is_applicable(&NetworkSetup::empty(), interface_inspector)
                .is_err()
        );
    }

    #[test]
    fn is_applicable_remove_interface() {
        let patch: NetworkSetupPatch = serde_json::from_value(json!(
            [
                {"op": "remove", "path": "/abmr1/lan1"}
            ]
        ))
        .unwrap();

        let network_setup: NetworkSetup = serde_json::from_value(json!(
            {
                "abmr1": {"lan1": {}}
            }
        ))
        .unwrap();

        let interface_inspector =
            |_: &MachineNameStr, _: &MachineNetworkInterfaceStr| ControlFlow::Continue(());

        assert!(
            patch
                .is_applicable(&network_setup, interface_inspector)
                .is_ok()
        );

        // Also check that the patch is not applicable if there is no entry for /abmr1/lan1
        assert!(
            patch
                .is_applicable(&NetworkSetup::empty(), interface_inspector)
                .is_err()
        );
    }

    #[test]
    fn is_applicable_remove_machine() {
        let patch: NetworkSetupPatch = serde_json::from_value(json!(
            [
                {"op": "remove", "path": "/abmr1"}
            ]
        ))
        .unwrap();

        let network_setup: NetworkSetup = serde_json::from_value(json!(
            {
                "abmr1": {"lan1": {}}
            }
        ))
        .unwrap();

        let interface_inspector =
            |_: &MachineNameStr, _: &MachineNetworkInterfaceStr| ControlFlow::Continue(());
        assert!(
            patch
                .is_applicable(&network_setup, interface_inspector)
                .is_ok()
        );

        // Also check that the patch is not applicable if there is no entry for /abmr1
        assert!(
            patch
                .is_applicable(&NetworkSetup::empty(), interface_inspector)
                .is_err()
        );
    }

    #[test]
    fn is_applicable_add_machine_then_remove_machine() {
        let patch: NetworkSetupPatch = serde_json::from_value(json!(
            [
                {"op": "add", "path": "/abmr1", "value": {"lan1": {}}},
                {"op": "remove", "path": "/abmr1"}
            ]
        ))
        .unwrap();

        assert!(
            patch
                .is_applicable(&NetworkSetup::empty(), |_, _| ControlFlow::Continue(()))
                .is_ok()
        );
    }

    #[test]
    fn is_applicable_add_machine_then_remove_interface() {
        let patch: NetworkSetupPatch = serde_json::from_value(json!(
            [
                {"op": "add", "path": "/abmr1", "value": {"lan1": {}}},
                {"op": "remove", "path": "/abmr1/lan1"}
            ]
        ))
        .unwrap();

        assert!(
            patch
                .is_applicable(&NetworkSetup::empty(), |_, _| ControlFlow::Continue(()))
                .is_ok()
        );
    }

    #[test]
    fn is_applicable_user_defined_failure() {
        let patch: NetworkSetupPatch = serde_json::from_value(json!(
            [
                {"op": "add", "path": "/abmr1", "value": {"lan1": {}}}
            ]
        ))
        .unwrap();

        assert!(
            patch
                .is_applicable(&NetworkSetup::empty(), |_, _| ControlFlow::Break(()))
                .is_err()
        );
    }
}
