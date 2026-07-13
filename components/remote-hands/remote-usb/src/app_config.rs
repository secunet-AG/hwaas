// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use serde::Deserialize;

#[derive(Deserialize)]
/// AppConfig: Only contains the path to the images.
/// Is needed for parsing the initial config provided to the app.
pub struct AppConfig {
    pub images_path: String,
}
