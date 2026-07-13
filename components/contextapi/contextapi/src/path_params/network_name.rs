// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Extract the `network` path param via [`axum::extract::Path`].
///
/// ## Example
/// see example of [`crate::extract::context_id::PathWithContextId`]
#[derive(Deserialize, Serialize, Debug, JsonSchema)]
pub(crate) struct PathParamsNetwork {
    /// a user chosen name representing a L2 network within a context
    pub(crate) network: String,
}
