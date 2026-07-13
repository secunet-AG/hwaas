// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, sync::Arc};

use crate::serial::serial_task::SerialTasks;

#[derive(Clone)]
/// AppState: Contains a HashMap of `SerialTasks`s.
pub struct AppState {
    pub serials: Arc<HashMap<String, SerialTasks>>,
}

impl Default for AppState {
    /// Initialization with an empty Hashmap.
    /// Is needed for OpenAPI spec generation, see `openapi-generator.rs`.
    fn default() -> Self {
        AppState {
            serials: Arc::new(HashMap::new()),
        }
    }
}

impl AppState {
    /// Initialization with given serial state.
    pub fn new(serials: HashMap<String, SerialTasks>) -> Self {
        AppState {
            serials: Arc::new(serials),
        }
    }
}
