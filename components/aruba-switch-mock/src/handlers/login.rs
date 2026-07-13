// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::app_state::AppState;
use crate::credential_cookie_name::CREDENTIAL_COOKIE_NAME;
use aruba_structs::login_sessions::{RestLoginSessions, RestLoginSessionsResult};
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use std::sync::Arc;
use tracing::{debug, error, info};
use uuid::Uuid;

pub(crate) async fn login(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RestLoginSessions>,
) -> Result<Json<RestLoginSessionsResult>, (StatusCode, &'static str)> {
    if !state.logins.contains_key(&payload) {
        return Err((StatusCode::FORBIDDEN, "Invalid login"));
    }

    let cookie = Uuid::new_v4().to_string();
    debug!("Generate new cookie: '{}'", cookie);

    // try insert
    state.logins.alter(&payload, |_, mut v| {
        if v.len() < state.max_sessions {
            v.push(cookie.clone())
        }

        v
    });

    // test if insert was possible
    let res = state
        .logins
        .get(&payload)
        .ok_or_else(|| {
            error!("Could not get logins");
            (StatusCode::INTERNAL_SERVER_ERROR, "AppState error")
        })?
        .contains(&cookie);

    let cookie = format!("{}={}", CREDENTIAL_COOKIE_NAME, cookie);

    match res {
        true => {
            info!("Logged in {}", payload.user_name);
            Ok(Json::from(RestLoginSessionsResult { cookie }))
        }
        false => Err((StatusCode::FORBIDDEN, "too many sessions open")),
    }
}
