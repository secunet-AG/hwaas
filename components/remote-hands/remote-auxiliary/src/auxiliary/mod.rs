// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::app_config::AuxJsonConfig;
use serde::Deserialize;

#[derive(Clone, Copy, Debug)]
/// The `AuxState` a machine can have: `On` or `Off`.
pub enum AuxState {
    On,
    Off,
}

impl AuxState {
    /// Helper function for API, to check if current state is desired state already.
    pub fn match_bool(&self, state: bool) -> bool {
        match self {
            AuxState::On => state,
            AuxState::Off => !state,
        }
    }
}

#[derive(Clone, Deserialize)]
/// Static config of one auxiliary device.
/// Contains an ID/name, the url under which the auxiliary device is reachable
/// and a command for (de-)activation.
pub struct AuxConfig {
    pub id: String,
    pub url: String,
    pub cmd: String,
}

/// Function to convert a `AuxJsonConfig` into a `AuxConfig`.
/// Used in main() to transform input `AppConfig` information into internal
/// `AppState` information.
pub fn new(aux_config: AuxJsonConfig) -> AuxConfig {
    let config: AuxConfig = serde_json::from_value(aux_config.config).expect("auxiliary config");
    AuxConfig {
        id: config.id,
        url: config.url,
        cmd: config.cmd,
    }
}
