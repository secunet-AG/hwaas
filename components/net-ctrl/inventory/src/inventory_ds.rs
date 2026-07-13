// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use network_type_ids::{SwitchDetails, SwitchID};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use switch::SwitchModel;

/// Container for the information to construct a new SwitchAPI.
/// The [`SwitchModel`] is used for determining the correct SwitchAPI type.
/// The [`SwitchDetails`] contains all information needed by one SwitchAPI.
#[derive(Debug, Clone, JsonSchema, Serialize, Deserialize, Eq, PartialEq)]
pub struct SwitchModelDetail {
    pub model: SwitchModel,

    #[serde(flatten)]
    pub details: SwitchDetails,
}

pub type SwitchMapping = HashMap<SwitchID, SwitchModelDetail>;
