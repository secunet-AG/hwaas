// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use axum::extract::FromRef;

use connection_handler::ConnectionHandler;

/// full state of the app
/// Handlers receive a clone of this state from the Axum framework.
/// Hence, for mutable state, it has to be ensured it is always in sync for all clones over all tasks.
/// Read-only state (like configs) is simply cloned to avoid looking for thread-safety.
#[derive(Clone)]
pub struct AppState {
    /// switch connection handler
    pub connection_handler: Arc<ConnectionHandler>,
}

// support converting an `AppState` in an `Arc<ConnectionHandler>`
impl FromRef<AppState> for Arc<ConnectionHandler> {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.connection_handler.clone()
    }
}
