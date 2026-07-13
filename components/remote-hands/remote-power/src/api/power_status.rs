// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use schemars::JsonSchema;
use serde::Serialize;

#[derive(Clone, Debug, Serialize, JsonSchema)]
/// User-facing meta information to return as responses for requests
pub struct PowerStatus {
    power_id: String,
    state: bool,
}

impl PowerStatus {
    /// User-facing meta information for the given power id/name with state
    /// set to `true` to represent the power interface is on.
    pub fn on(power_id: String) -> Self {
        PowerStatus {
            power_id,
            state: true,
        }
    }

    /// User-facing meta information for the given power id/name with state
    /// set to `false` to represent the power interface is off.
    pub fn off(power_id: String) -> Self {
        PowerStatus {
            power_id,
            state: false,
        }
    }
}
