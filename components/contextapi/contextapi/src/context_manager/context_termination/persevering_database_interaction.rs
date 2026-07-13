// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

//! This module contains functionality for interacting with the database
//! in a way which one normally would not need, but is desired when terminating
//! contexts as that is an operation we want to (eventually) succeed.
use db_interaction::connection::DbFacade;
use diesel::SqliteConnection;
use rand::Rng;
use std::time::Duration;
use tracing::{debug, error, instrument};

/// Calculates the sleep before the next database retry
/// can take place.
#[instrument]
async fn retry_sleep(exponent: u32) {
    let backoff = 2_u64.pow(exponent);
    let jitter = backoff + rand::thread_rng().gen_range(0..backoff);
    let duration = Duration::from_millis(jitter);
    debug!(milliseconds = duration.as_millis(), "sleeping");
    let _ = tokio::time::sleep(duration).await;
}

/// Extension trait for conveniently interacting with the database with retries.
#[trait_variant::make(Send)]
pub(super) trait InteractWithRetriesExt {
    async fn read_with_retries<F, T>(&self, f: F) -> T
    where
        T: Send + 'static,
        F: Clone + Send + Sync + 'static,
        F: Fn(&mut SqliteConnection) -> Result<T, diesel::result::Error> + Send + 'static;

    async fn write_with_retries<F, T>(&self, f: F) -> T
    where
        T: Send + 'static,
        F: Send + Clone + 'static,
        F: Fn(&mut SqliteConnection) -> Result<T, diesel::result::Error>;
}

impl InteractWithRetriesExt for DbFacade {
    /// Attempts to spawn a call to read from the database. The user is responsible
    /// for ensuring that the provided closure does not perform any writes.
    ///
    /// The call is retried until it succeeds with increasingly long sleeps
    /// between retries. The caller is thus adviced to wrap this in a timeout in order to
    /// avoid waiting potentially indefinitely long.
    ///
    /// # Warning
    ///
    /// This should not be used for database writes.
    ///
    /// If the returned future is dropped there is no guarantee that the spawned database
    /// interaction does not take place in the future.
    #[instrument(skip_all)]
    async fn read_with_retries<F, T>(&self, f: F) -> T
    where
        T: Send + 'static,
        F: Clone + Send + Sync + 'static,
        F: Fn(&mut SqliteConnection) -> Result<T, diesel::result::Error> + Send + 'static,
    {
        let mut retry_attempts = 0;
        let mut backoff_exponent = 1;
        loop {
            debug!(
                attempt_number = retry_attempts,
                "attempting to interact with the database"
            );
            match self.spawn_call(f.clone()).await.inspect_err(|e| error!(attempt_number = retry_attempts + 1,error.dbg = ?e, error.msg = %e, error.sources = error_utils::source_chain(e), "database read failed")) {

                Ok(value) => {
                    return value;
                }
                _ => {
                    // We do not log an error as we expect that to be done by `f`
                    retry_attempts += 1;
                    retry_sleep(backoff_exponent).await;
                    backoff_exponent += 1;
                }
            }
        }
    }

    /// Attempts to call a database call involving one or more writes.
    ///
    /// The call is retried until it succeeds with increasingly long
    /// sleeps in between retries. Thus the caller is adviced to wrap this
    /// in a timeout to avoid waiting indefinitely long.
    ///
    /// # Cancellation safety
    ///
    /// The thread is blocked for the duration of each attempt to call `f` hence
    /// if the future is dropped at an await point it is safe to assume that the
    /// provided `f` will not run at a later point. Thus if `f` represents a
    /// transaction we know that it was only applied if the future was successfully
    /// resolved.
    #[instrument(skip_all)]
    async fn write_with_retries<F, T>(&self, f: F) -> T
    where
        T: Send + 'static,
        F: Send + Clone + 'static,
        F: Fn(&mut SqliteConnection) -> Result<T, diesel::result::Error>,
    {
        let mut retry_attempts = 0;
        let mut backoff_exponent = 1;
        loop {
            debug!(
                attempt_number = retry_attempts,
                "attempting to interact with the database"
            );

            if let Ok(value) = self.execute_on_current_thread(f.clone()).await.inspect_err(|e| error!(attempt_number = retry_attempts + 1, error.dbg = ?e, error.msg = %e, error.sources = error_utils::source_chain(e), "database write failed")) {
                return value;
            } else {
                // We do not log an error as we expect that to be done by `f`
                retry_attempts += 1;
                retry_sleep(backoff_exponent).await;
                backoff_exponent += 1;
            }
        }
    }
}
