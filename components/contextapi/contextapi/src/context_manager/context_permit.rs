// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use db_interaction::models::context_id::ContextIdBytes;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_util::sync::{CancellationToken, WaitForCancellationFutureOwned};

/// Represents permission to use the resources assigned to a
/// context.
///
/// Handlers that work with context resource's should obtain these
/// through an axum extractor.
pub(crate) struct ContextAccessToken {
    _permit: OwnedSemaphorePermit,
    pub(crate) context_id: ContextIdBytes,
    derived_token: CancellationToken,
}

// Prevents destruction through destructuring.
impl Drop for ContextAccessToken {
    fn drop(&mut self) {}
}

impl ContextAccessToken {
    /// Returns a future that completes when the context manager initiates
    /// termination of the corresponding context.
    ///
    /// This consumes the context access token because termination may never
    /// occur while the token is still being held.
    pub(crate) fn cancelled_owned(self) -> WaitForCancellationFutureOwned {
        self.derived_token.clone().cancelled_owned()
    }
}

impl std::fmt::Debug for ContextAccessToken {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(
            fmt,
            "ContextAccessToken {{ context_id: {:?}, derived_token: {:?} }}",
            self.context_id, self.derived_token
        )
    }
}

/// Provides a single [`ContextAccessToken`] through the
/// [`Self::acquire`] method which waits until the last
/// granted access token is dropped before returning.
pub(crate) struct ContextAccess(Arc<Semaphore>, CancellationToken);
impl ContextAccess {
    pub(super) fn new() -> Self {
        // We only allow one permit to limit access the same way a mutex would.
        let semaphore = Semaphore::new(1);
        Self(Arc::new(semaphore), CancellationToken::new())
    }

    /// Waits until the exclusive access becomes available.
    pub(super) fn acquire(
        &self,
        context_id: ContextIdBytes,
    ) -> impl Future<Output = ContextAccessToken> + Send + 'static {
        let fut = self.0.clone().acquire_owned();
        let derived_token = self.1.clone();
        async move {
            ContextAccessToken {
                // expect is OK because there is no way to call .close on the inner semaphore.
                _permit: fut.await.expect("The inner semaphore should not be closed"),
                context_id,
                derived_token,
            }
        }
    }
}
