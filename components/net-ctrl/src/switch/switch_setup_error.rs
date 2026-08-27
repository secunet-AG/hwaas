// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::network_type_ids::{PortID, VlanID};
use std::error::Error;
use std::fmt::{Display, Formatter};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum SwitchSetupError {
    DestinationUnreachable,
    UnexpectedResponseFromSwitch,
    InternalError,
    VlanIdSetupError(VlanID),
    TrunkTaggedVlanSetupError(PortID, VlanID),
    CriticalPortSetupError(PortID),
}

impl Display for SwitchSetupError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for SwitchSetupError {}
