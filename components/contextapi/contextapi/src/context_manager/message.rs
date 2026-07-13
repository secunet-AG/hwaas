// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use super::context_permit::ContextAccessToken;
use db_interaction::models::context_id::ContextIdBytes;
use tokio::sync::oneshot::Sender as OneShotSender;

/// A message to interact with the context manager.
pub(crate) enum ContextManagerMessage {
    /// Indicates that a new context has been created.
    New {
        ctx_id: ContextIdBytes,
        timeout: Duration,
    },
    /// Indicates a desire to delete a context.
    Delete(ContextIdBytes),
    /// Indicates a desire to receive a permit to work
    /// with a context.
    GetPermit {
        ctx_id: ContextIdBytes,
        return_with: OneShotSender<Option<ContextAccessToken>>,
    },
    /// Set a new timeout for the context.
    Timeout {
        ctx_id: ContextIdBytes,
        timeout: Duration,
    },
}
