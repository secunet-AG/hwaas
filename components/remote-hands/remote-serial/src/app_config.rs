// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
/// Only temporarily needed in main() for parsing of the AppConfig.
/// Will be transformed to `SerialState` before adding to the `AppState`.
pub struct SerialConfig {
    #[serde(rename = "type")]
    pub serial_type: String,
}

#[derive(Deserialize)]
/// AppConfig: Contains a HashMap of `SerialConfig`s as JSON.
/// Is needed for parsing the initial config provided to the app.
pub struct AppConfig {
    pub serials: HashMap<String, serde_json::Value>,
}
