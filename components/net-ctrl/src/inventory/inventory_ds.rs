// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::network_type_ids::{SwitchDetails, SwitchID};
use crate::switch::SwitchModel;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Container for the information to construct a new SwitchAPI.
/// The [`SwitchModel`] is used for determining the correct SwitchAPI type.
/// The [`SwitchDetails`] contains all information needed by one SwitchAPI.
#[derive(Debug, Clone, JsonSchema, Serialize, Deserialize, Eq, PartialEq)]
#[schemars(
    description = "Configuration details for a switch. The model identifies \
                   the switch implementation; the remaining fields contain \
                   the connection and VLAN configuration."
)]
pub struct SwitchModelDetail {
    pub model: SwitchModel,

    #[serde(flatten)]
    pub details: SwitchDetails,
}

pub type SwitchMapping = HashMap<SwitchID, SwitchModelDetail>;
