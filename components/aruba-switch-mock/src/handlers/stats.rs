// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::app_state::{AppState, AppStateStats};
use axum::Json;
use axum::extract::State;
use std::sync::Arc;

pub(crate) async fn stats(State(state): State<Arc<AppState>>) -> Json<AppStateStats> {
    Json(state.stats.clone())
}
