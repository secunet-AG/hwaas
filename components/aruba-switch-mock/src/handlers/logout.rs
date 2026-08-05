// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::app_state::AppState;
use crate::credential_cookie_name::CREDENTIAL_COOKIE_NAME;
use aruba_structs::login_sessions::RestLoginSessions;
use axum::Extension;
use axum::extract::State;
use axum::http::StatusCode;
use std::sync::Arc;
use tower_cookies::Cookies;
use tracing::{debug, info};

pub(crate) async fn logout(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<RestLoginSessions>,
    cookies: Cookies,
) -> Result<(), (StatusCode, &'static str)> {
    let c = cookies
        .get(CREDENTIAL_COOKIE_NAME)
        .ok_or((StatusCode::UNAUTHORIZED, "Missing cookie"))?;

    debug!("Found cookie '{}'", c);

    state.logins.alter_all(|_, mut v| {
        v.retain(|e| e != c.value());
        v
    });

    info!("Logout {} @ {}", user, c.value());

    Ok(())
}
