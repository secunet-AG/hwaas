// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;

use assert_fs::TempDir;

use diesel::prelude::*;

use crate::connection::ProvisionalDbConnection;

/// A database for testing purposes.
pub struct TestDb {
    _directory: TempDir,
    pub file: PathBuf,
    pub conn: SqliteConnection,
}

/// Prevent _directory from being dropped when destructuring,
/// by preventing destructuring.
impl Drop for TestDb {
    fn drop(&mut self) {}
}

impl TestDb {
    /// Creates a database for testing purposes.
    ///
    /// The database is initialized with the migrations from the
    /// migrations directory.
    pub fn spawn() -> Self {
        let directory =
            TempDir::new().expect("Should be possible to create temporary directory for testing");
        let db_file = directory.path().join("test.db");
        let conn =
            ProvisionalDbConnection::new(db_file.to_str().expect("Path should be valid unicode"))
                .expect("Should be possible to connect to test database")
                .configured()
                .expect("Should be possible to configure test database");

        TestDb {
            file: db_file,
            _directory: directory,
            conn,
        }
    }
}
