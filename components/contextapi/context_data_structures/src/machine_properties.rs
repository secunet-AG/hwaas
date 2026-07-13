// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The properties of a Machine.
#[derive(Debug, Clone, JsonSchema, Serialize, Deserialize)]
pub struct MachineProperties {
    /// The platform of the Machine.
    pub platform: String,
}

/// Information about the Machine.
#[derive(Debug, Clone, JsonSchema, Serialize, Deserialize)]
pub struct MachineInfo {
    /// The id of the Machine.
    pub id: i32,
    /// The platform of the Machine
    pub platform: String,
}
