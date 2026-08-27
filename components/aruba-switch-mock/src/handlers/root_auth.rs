// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::structs::login_sessions::RestLoginSessions;
use axum::Extension;
use axum::response::Html;

pub(crate) async fn handler_auth(Extension(login): Extension<RestLoginSessions>) -> Html<String> {
    Html(format!("<h1>Hello, {}!</h1>", login))
}
