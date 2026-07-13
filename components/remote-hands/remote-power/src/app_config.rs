// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
/// AppConfig: Contains a HashMap of `PowerControlConfig`s.
/// Is needed for parsing the initial config provided to the app.
pub struct AppConfig {
    pub controls: HashMap<String, PowerControlConfig>,
}

#[derive(Deserialize)]
/// One power config with a type and a config. The format of the config is
/// determined by the type. We currently only support "custom" as a type,
/// so the config is of type `CustomPowerConfig`.
pub struct PowerControlConfig {
    #[serde(rename = "type")]
    pub power_type: String,
    pub config: serde_json::Value,
}
