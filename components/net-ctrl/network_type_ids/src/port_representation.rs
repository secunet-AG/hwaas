// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::PortID;
use schemars::JsonSchema;
use serde::Serialize;
use std::fmt::{Display, Formatter};

// Maintains basic info about a Port on a specific switch. Ports are only locally unique.
#[derive(Serialize, JsonSchema, Debug, PartialEq, Eq, PartialOrd)]
pub struct PortRepresentation {
    /// Local ID of a port.
    pub port_id: PortID,
}

impl Display for PortRepresentation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl PortRepresentation {
    pub fn new(port_id: PortID) -> Self {
        Self { port_id }
    }
}
