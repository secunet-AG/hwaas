// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use schemars::JsonSchema;
use serde::Serialize;

#[derive(Clone, Debug, Serialize, JsonSchema)]
/// User-facing meta information to return as responses for requests
pub struct AuxiliaryDevice {
    id: String,
    activation: bool,
}

impl AuxiliaryDevice {
    /// Helper function to create an active `AuxiliaryDevice` object for the
    /// given device id to return to a users request. Also used to create
    /// examples for the OpenAPI documentation.
    pub fn on(id: &str) -> Self {
        AuxiliaryDevice {
            id: id.to_string(),
            activation: true,
        }
    }

    /// Helper function to create an inactive `AuxiliaryDevice` object for the
    /// given device id to return to a users request. Also used to create
    /// examples for the OpenAPI documentation.
    pub fn off(id: &str) -> Self {
        AuxiliaryDevice {
            id: id.to_string(),
            activation: false,
        }
    }
}
