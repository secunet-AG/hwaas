// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
/// AppConfig: Contains a HashMap of `AuxJsonConfig`s.
/// Is needed for parsing the initial config provided to the app.
pub struct AppConfig {
    #[serde(rename = "devices")]
    pub aux_devices: HashMap<String, AuxJsonConfig>,
}

#[derive(Deserialize)]
/// Only temporarely needed in main() for parsing of the AppConfig.
/// Will be transformed to `AuxConfig` before adding to the `AppState`.
pub struct AuxJsonConfig {
    pub config: serde_json::Value,
}
