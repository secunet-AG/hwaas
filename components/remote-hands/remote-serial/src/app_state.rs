// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, sync::Arc};

use crate::serial::SerialState;

#[derive(Clone)]
/// AppState: Contains a HashMap of `SerialState`s.
pub struct AppState {
    pub serials: Arc<HashMap<String, SerialState>>,
}

impl Default for AppState {
    /// Initilization with an empty Hashmap.
    /// Is needed for OpenAPI spec generation, see `openapi-generator.rs`.
    fn default() -> Self {
        AppState {
            serials: Arc::new(HashMap::new()),
        }
    }
}

impl AppState {
    /// Initilization with given serial state.
    pub fn new(serials: HashMap<String, SerialState>) -> Self {
        AppState {
            serials: Arc::new(serials),
        }
    }
}
