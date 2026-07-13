// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use axum::response::Html;

pub(crate) async fn handler() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}
