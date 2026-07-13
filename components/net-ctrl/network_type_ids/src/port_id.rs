// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};

/// Wrapper type for Port IDs. Can hold different ID types for different switch models.
/// Generic Port ID used for all switch models. Individual conversion should be handled in the switch APIs.
#[derive(JsonSchema, Debug, PartialEq, Eq, Hash, Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct PortID(String);

impl PortID {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn get(&self) -> &String {
        &self.0
    }

    pub fn escaped(&self) -> String {
        self.get().replace('/', "%2F")
    }
}

impl PartialOrd<Self> for PortID {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let num_self = self.get().parse::<usize>().ok()?;
        let num_other = other.get().parse::<usize>().ok()?;
        Some(num_self.cmp(&num_other))
    }
}

impl Display for PortID {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.get().as_str())
    }
}

impl From<usize> for PortID {
    fn from(i: usize) -> Self {
        PortID(i.to_string())
    }
}

impl From<String> for PortID {
    fn from(s: String) -> Self {
        PortID(s)
    }
}
