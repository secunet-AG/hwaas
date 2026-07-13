// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

pub mod api;
pub mod app_config;
pub mod power;

use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

#[derive(Clone)]
/// AppState: Contains a HashMap of `PowerControlBackend`s.
/// Encapsulated in a Mutex since we don't want to trigger power commands in
/// parallel, since they are hardware-dependent and potentially cannot buffer
/// requests.
pub struct AppState {
    #[allow(clippy::type_complexity)]
    pub controls: Arc<HashMap<String, Arc<Mutex<power::PowerControlBackend>>>>,
}
