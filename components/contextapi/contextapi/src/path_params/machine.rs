// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use context_data_structures::aliases::MachineName;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Extract the path params via [`axum::extract::Path`].
///
/// ## Example
/// see example of [`crate::extract::context_id::PathWithContextId`]
#[derive(Deserialize, Serialize, JsonSchema)]
pub(crate) struct PathParamsMachineName {
    /// name of the machine
    pub(crate) machine_name: MachineName,
}
