// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Debug;
use std::sync::Arc;

use deadpool_diesel::InteractError;
use deadpool_diesel::sqlite::{BuildError, Hook, HookError, Manager, Object, Pool};
use diesel::{Connection, SqliteConnection, connection::SimpleConnection};
use diesel_migrations::EmbeddedMigrations;
use diesel_migrations::MigrationHarness;
use either::Either;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

const MIGRATIONS: EmbeddedMigrations =
    diesel_migrations::embed_migrations!("./../db_interaction/migrations");
/// A database connection that has not yet been configured.
pub struct ProvisionalDbConnection(SqliteConnection);

/// An error that may occur while running pending migrations.
#[derive(Debug)]
pub struct MigrationError(Box<dyn std::error::Error + Send + Sync>);

impl std::fmt::Display for MigrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "migrations failed")
    }
}

impl std::error::Error for MigrationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.0.as_ref())
    }
}

impl ProvisionalDbConnection {
    /// Create a new unconfigured database connection.
    pub fn new(db_url: &str) -> Result<Self, diesel::ConnectionError> {
        SqliteConnection::establish(db_url).map(Self)
    }

    /// Returns a configured database connection. This includes
    /// setting all necessary pragmas as well as running all
    /// pending migrations.
    pub fn configured(
        mut self,
    ) -> Result<SqliteConnection, Either<diesel::result::Error, MigrationError>> {
        Self::set_pragmas(&mut self.0).map_err(Either::Left)?;
        self.0
            .run_pending_migrations(MIGRATIONS)
            .map_err(MigrationError)
            .map_err(Either::Right)?;
        Ok(self.0)
    }

    /// Enable foreign key support and retry getting locks for up to 100 milliseconds before returning an error.
    fn set_pragmas(conn: &mut SqliteConnection) -> Result<(), diesel::result::Error> {
        conn.batch_execute(
            "PRAGMA foreign_keys = ON; PRAGMA busy_timeout=100; PRAGMA journal_mode=WAL;",
        )
    }
}

/// Provides a streamlined way to interact with the database.
pub struct DbFacade {
    /// A configured pool of database connections.
    pool: Pool,
    permits: Arc<Semaphore>,
    num_permits: u32,
}

#[derive(Debug)]
pub struct DbFacadeConstructionError(Box<dyn std::error::Error + Send + Sync + 'static>);

impl DbFacadeConstructionError {
    fn from_error<E: std::error::Error + Send + Sync + 'static>(e: E) -> Self {
        Self(e.into())
    }
}

impl std::fmt::Display for DbFacadeConstructionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "failed to construct database facade")
    }
}

impl std::error::Error for DbFacadeConstructionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.0.as_ref())
    }
}
impl DbFacade {
    /// Creates a database facade that can be used to interact with the database.
    ///
    /// NOTE: This constructor runs all pending migrations before returning
    /// the [`DbFacade`].
    pub async fn new(db_url: &str, pool_size: u32) -> Result<Self, DbFacadeConstructionError> {
        let pool = establish_pool(db_url, pool_size as usize)
            .await
            .map_err(DbFacadeConstructionError::from_error)?;
        let permits = Arc::new(Semaphore::new(pool_size as usize));
        Ok(Self {
            pool,
            permits,
            num_permits: pool_size,
        })
    }
}

/// An error that may occur during asynchronous database reads.
#[derive(Debug)]
pub enum DatabaseInteractionError {
    CouldNotObtainConnection(deadpool_diesel::sqlite::PoolError),
    DbCallFailure(diesel::result::Error),
    SpawnedInteractionPanic,
    AbortedSpawnedInteraction,
    PoisonedMutex(Box<str>),
    /// This means that the database facade is closed which should only
    /// happen during App shutdown if at all.
    Closed,
}
impl std::fmt::Display for DatabaseInteractionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            Self::CouldNotObtainConnection(_) => "failed to get database connection",
            Self::DbCallFailure(_) => "the query failed",
            Self::SpawnedInteractionPanic => {
                "an unexpected panic occured while handling the asynchronous database read"
            }
            Self::AbortedSpawnedInteraction => "the asynchronous database read was aborted",
            Self::PoisonedMutex(_) => {
                "an unexpected panic has occurred which resulted in mutex poisoning"
            }
            Self::Closed => {
                "the database is permanently unavailable: semaphore closed: this is unexpected, but could happen during application shutdown."
            }
        };
        if let Self::PoisonedMutex(poison_error_msg) = self {
            write!(f, "{}: {}", msg, poison_error_msg)
        } else {
            write!(f, "{}", msg)
        }
    }
}
impl std::error::Error for DatabaseInteractionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CouldNotObtainConnection(err) => Some(err),
            Self::DbCallFailure(err) => Some(err),
            _ => None,
        }
    }
}

impl DbFacade {
    /// Spawn a call to the database on a non-blocking thread pool.
    ///
    /// # Important
    ///
    /// The caller must not perform writing actions in the given `query` as that can
    /// lead to (temporary) unexpected errors. See [`Self::execute_on_current_thread`]
    /// or [`Self::`spawn_writing_call`] if you want to write to the database.
    ///
    pub async fn spawn_call<T, F>(&self, query: F) -> Result<T, DatabaseInteractionError>
    where
        F: FnOnce(&mut SqliteConnection) -> diesel::QueryResult<T> + Send + 'static,
        T: Send + 'static,
    {
        self._spawn_call::<T, F, false>(query).await
    }

    /// # Important
    ///
    /// If the future is dropped the call may still be executed at a later point in
    /// the future. Hence this method should only be used if you have some other mechanism
    /// to ensure that the the write being executed outside of the current scope does not
    /// lead to unexpected behaviour.
    ///
    /// See also [`Self::execute_on_current_thread`] which may be a better alternative.
    pub async fn spawn_writing_call<T, F>(&self, query: F) -> Result<T, DatabaseInteractionError>
    where
        F: FnOnce(&mut SqliteConnection) -> diesel::QueryResult<T> + Send + 'static,
        T: Send + 'static,
    {
        self._spawn_call::<T, F, true>(query).await
    }

    async fn _spawn_call<T, F, const IS_WRITE: bool>(
        &self,
        query: F,
    ) -> Result<T, DatabaseInteractionError>
    where
        F: FnOnce(&mut SqliteConnection) -> diesel::QueryResult<T> + Send + 'static,
        T: Send + 'static,
    {
        // Get one or all permits to use the database, depending on whether we want to write or only read
        let guard: OwnedSemaphorePermit = {
            if IS_WRITE {
                self.permits
                    .clone()
                    .acquire_many_owned(self.num_permits)
                    .await
                    .map_err(|_| DatabaseInteractionError::Closed)?
            } else {
                self.permits
                    .clone()
                    .acquire_owned()
                    .await
                    .map_err(|_| DatabaseInteractionError::Closed)?
            }
        };
        let conn = self.get_connection().await?;
        // Move permit into the query: This ensures the permit gets dropped after the query has returned
        let query = move |conn: &mut SqliteConnection| {
            let _guard = guard;
            query(conn)
        };
        let query_result = conn.interact(query).await.map_err(|e| match e {
            InteractError::Panic(_) => DatabaseInteractionError::SpawnedInteractionPanic,
            InteractError::Aborted => DatabaseInteractionError::AbortedSpawnedInteraction,
        })?;
        query_result.map_err(DatabaseInteractionError::DbCallFailure)
    }
    /// Calls the database on the current thread.
    ///
    /// The method is async because we need to wait for a potentially long time
    /// to obtain a database connection before we can execute the given query.
    ///
    /// Use this method if you want the guarantee that your database query
    /// either gets executed in the current task or not at all. If this is not
    /// important then [`Self::spawn_call`] should be preferred.
    pub async fn execute_on_current_thread<T, F>(
        &self,
        query: F,
    ) -> Result<T, DatabaseInteractionError>
    where
        F: FnOnce(&mut SqliteConnection) -> diesel::QueryResult<T>,
    {
        // Acquire all permits to make sure there are no other open transactions
        // while we perform our write(s).
        let _guard = self
            .permits
            .acquire_many(self.num_permits)
            .await
            .map_err(|_| DatabaseInteractionError::Closed)?;
        let conn = self.get_connection().await?;
        let mut guard = conn
            .lock()
            .map_err(|e| DatabaseInteractionError::PoisonedMutex(format!("{}", e).into()))?;
        query(&mut guard).map_err(DatabaseInteractionError::DbCallFailure)
    }

    async fn get_connection(&self) -> Result<Object, DatabaseInteractionError> {
        self.pool
            .get()
            .await
            .map_err(DatabaseInteractionError::CouldNotObtainConnection)
    }
}

/// The error type which may occur when attempting to build a database pool.
pub type PoolSetupError = Either<BuildError, MigrationError>;

/// Establishes a configured connection pool for the Sqlite database.
///
/// NOTE: This runs all pending migrations before returning the pool.
async fn establish_pool(
    database_url: &str,
    max_connections: usize,
) -> Result<Pool, PoolSetupError> {
    let manager = Manager::new(database_url, deadpool_diesel::Runtime::Tokio1);
    let pool = Pool::builder(manager)
        .max_size(max_connections)
        .post_create(Hook::async_fn(|conn, _| {
            Box::pin(async {
                match conn.interact(ProvisionalDbConnection::set_pragmas).await {
                    Err(interaction_error) => Err(HookError::message(format!(
                        "connection interaction failed: {:?}",
                        interaction_error
                    ))),
                    Ok(result) => {
                        result.map_err(|e| HookError::Backend(deadpool_diesel::Error::Ping(e)))
                    }
                }
            })
        }))
        .build()
        .map_err(Either::Left)?;
    let conn = pool
        .get()
        .await
        .map_err(|e| MigrationError(e.into()))
        .map_err(Either::Right)?;

    let mut conn_guard = conn
        .try_lock()
        .expect("Should have exclusive access to the pool");
    conn_guard
        .as_mut()
        .run_pending_migrations(MIGRATIONS)
        .map_err(MigrationError)
        .map_err(Either::Right)?;
    Ok(pool)
}
