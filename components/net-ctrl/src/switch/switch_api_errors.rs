// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::error::Error;
use std::fmt::{Display, Formatter};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum SwitchApiError {
    Unauthorized,
    DestinationUnreachable,
    IDInvalid,
    UnexpectedResponseFromSwitch,
    BuiltFaultyRequestToSwitch,
}

impl Error for SwitchApiError {}

impl Display for SwitchApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
