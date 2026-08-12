// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

mod connection_handler_error;
mod connection_handler_impl;

pub use crate::inventory::SwitchMapping;
pub use connection_handler_error::ConnectionHandlerError;
pub use connection_handler_impl::ConnectionHandler;
