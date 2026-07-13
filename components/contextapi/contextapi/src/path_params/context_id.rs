// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use context_data_structures::aliases::ContextId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Extract the ContextId via [`axum::extract::Path`].
///
/// ## Example
/// ```no-test
/// fn my_handler(Path(PathParamsContextId { ctx_id }): Path<PathParamsContextId>) {}
/// ```
#[derive(Deserialize, Serialize, Debug, JsonSchema)]
pub(crate) struct PathParamsContextId {
    /// Context access token
    pub(crate) ctx_id: ContextId,
}
