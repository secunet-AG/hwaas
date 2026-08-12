// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::switch::SwitchApiError;
use std::error::Error;
use std::fmt::{Display, Formatter};
use system_error::Error as SysError;

#[derive(Debug)]
pub enum ConnectionHandlerError {
    SwitchNotFound,
    ConnectionSetupFailed(SwitchApiError),
    InvalidCacheEntry,
    EntryGone,
    System(SysError),
}

impl Error for ConnectionHandlerError {}

impl Display for ConnectionHandlerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl From<SwitchApiError> for ConnectionHandlerError {
    fn from(e: SwitchApiError) -> Self {
        ConnectionHandlerError::ConnectionSetupFailed(e)
    }
}

impl From<SysError> for ConnectionHandlerError {
    fn from(e: SysError) -> Self {
        ConnectionHandlerError::System(e)
    }
}
