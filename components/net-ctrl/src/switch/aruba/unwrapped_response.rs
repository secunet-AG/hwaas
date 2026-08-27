// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use http::StatusCode;

/// carries the important information of a received [`reqwest::Response`].
#[derive(Debug, Clone)]
pub(super) struct UnwrappedResponse {
    pub(super) status_code: StatusCode,
    pub(super) body: String,
}
