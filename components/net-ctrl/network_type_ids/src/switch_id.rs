// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use schemars::JsonSchema;
#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::str::FromStr;

/// Contains information used to globally identify a switch.
#[derive(JsonSchema, Hash, Eq, PartialEq, Clone, Debug)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct SwitchID(String);

impl SwitchID {
    pub fn new(id: String) -> Self {
        Self(id)
    }
}

impl Display for SwitchID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for SwitchID {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_string()))
    }
}
