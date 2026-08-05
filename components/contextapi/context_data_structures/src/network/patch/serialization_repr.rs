// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

//! This module contains internal types that serialize to and
//! deserialize from operations described in a network setup JSON patch in
//! accordance with the JSON patch specification.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::network::{MachineInterfaceSet, machine_interface_set::EmptyMap};

use super::{
    AddOp, RemoveOp,
    json_ptr::{InterfaceJsonPtr, MachineJsonPtr},
};

/// `NetworkSetup` JSON Patch remove operation.
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "RemoveOp")]
pub(super) enum RemoveOpInner {
    InterfaceJsonPtr { path: InterfaceJsonPtr },
    MachineJsonPtr { path: MachineJsonPtr },
}

impl From<RemoveOpInner> for RemoveOp {
    fn from(value: RemoveOpInner) -> Self {
        use RemoveOpInner::*;
        match value {
            InterfaceJsonPtr { path } => Self::TaggedInterface(path.0),
            MachineJsonPtr { path } => Self::Machine(path.0),
        }
    }
}

impl From<RemoveOp> for RemoveOpInner {
    fn from(value: RemoveOp) -> Self {
        use RemoveOp::*;
        match value {
            TaggedInterface(tagged_interface) => RemoveOpInner::InterfaceJsonPtr {
                path: InterfaceJsonPtr(tagged_interface),
            },
            Machine(machine_id) => RemoveOpInner::MachineJsonPtr {
                path: MachineJsonPtr(machine_id),
            },
        }
    }
}

/// `NetworkSetup` JSON Patch add operation.
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "AddOp")]
pub(super) enum AddOpInner {
    TaggedInterface {
        path: InterfaceJsonPtr,
        value: EmptyMap,
    },
    Machine {
        path: MachineJsonPtr,
        value: MachineInterfaceSet,
    },
}

impl From<AddOp> for AddOpInner {
    fn from(input: AddOp) -> Self {
        match input {
            AddOp::Interface(interface) => AddOpInner::TaggedInterface {
                path: InterfaceJsonPtr(interface),
                value: EmptyMap(()),
            },
            AddOp::MachineWithInterfaces {
                machine,
                interfaces,
            } => AddOpInner::Machine {
                path: MachineJsonPtr(machine),
                value: interfaces,
            },
        }
    }
}

impl From<AddOpInner> for AddOp {
    fn from(input: AddOpInner) -> Self {
        match input {
            AddOpInner::TaggedInterface { path, .. } => AddOp::Interface(path.0),
            AddOpInner::Machine { path, value } => AddOp::MachineWithInterfaces {
                machine: path.0,
                interfaces: value,
            },
        }
    }
}
