// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::app_state::AppState;
use crate::credential_cookie_name::CREDENTIAL_COOKIE_NAME;
use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use std::sync::Arc;
use tower_cookies::Cookies;
use tracing::{debug, error, warn};

pub(crate) async fn check_auth(
    State(state): State<Arc<AppState>>,
    cookies: Cookies,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    let err_response = (StatusCode::UNAUTHORIZED, "Missing cookie").into_response();
    let c = cookies.get(CREDENTIAL_COOKIE_NAME).ok_or(());

    match c {
        Err(_) => {
            error!("No cookie found for '{}'", CREDENTIAL_COOKIE_NAME);

            let _ = cookies
                .list()
                .iter()
                .map(|c| debug!("Did you mean: '{:?}'", c));

            err_response
        }
        Ok(c) => {
            match state
                .logins
                .clone()
                .into_read_only()
                .iter()
                .find(|(_, cookies)| cookies.contains(&c.value().to_string()))
            {
                None => {
                    warn!("Cookie '{}' does not match any login", c.value());
                    err_response
                }
                Some((l, _)) => {
                    debug!("User is authorized as '{}'", l.user_name);

                    let _ = req
                        .extensions_mut()
                        .insert(l.clone())
                        .map(|v| warn!("RestLoginSession extension already registered: {}", v));

                    next.run(req).await
                }
            }
        }
    }
}
