// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::aliases::MachineName;

/// Resource setup description
#[derive(Debug, Clone, JsonSchema, Serialize, Deserialize, Eq, PartialEq)]
pub struct Rsd {
    pub machines: HashMap<MachineName, ResourceConstraints>,
}

/// Properties a suitable machine should satisfy.
#[derive(Debug, Clone, JsonSchema, Serialize, Deserialize, Eq, PartialEq)]
pub struct ResourceConstraints {
    /// The platform. This field must always be set.
    pub platform: String,
    /// The machine's unique id. This field is NOT recommended,
    /// but may be needed for certain use cases.
    // NOTE: We display the type as if it was a u16 in the json schema,
    // because we will not use negative values in practice, but need i32
    // for type compatibility with the Database.
    #[schemars(with = "Option<u16>")]
    pub machine_id: Option<i32>,
}
