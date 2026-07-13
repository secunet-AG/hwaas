// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Copy, Clone, Debug, Deserialize, Serialize, Eq, Hash, PartialEq, JsonSchema)]
pub struct NetworkSelector(u16);

impl Display for NetworkSelector {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}

impl NetworkSelector {
    pub fn new(id: u16) -> Self {
        Self(id)
    }
    pub fn get_id(&self) -> u16 {
        self.0
    }
}
