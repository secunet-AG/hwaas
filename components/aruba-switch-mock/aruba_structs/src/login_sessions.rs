// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Deserialize, Serialize, JsonSchema, Eq, PartialEq, Hash, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RestLoginSessions {
    /// The User Name of the user.
    pub user_name: String,

    /// The Password for the user.
    pub password: String,
}

impl Display for RestLoginSessions {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("User:{}", self.user_name))
    }
}

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RestLoginSessionsResult {
    /// The User login session id.
    pub cookie: String,
}
