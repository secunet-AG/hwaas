#![allow(
    dead_code,
    reason = "this is shared test code, but clippy doesn't understand this"
)]

use anyhow::Context as _;
use assert_fs::fixture::PathChild as _;
use core::pin::Pin;
use futures::task::Context;
use futures::task::Poll;
use futures::FutureExt as _;
use image_api::ImageHandler;
use std::cell::LazyCell;
use std::panic::UnwindSafe;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::ReadBuf;

/// A simple stream impl to mock image contents passed in from web forms.
pub struct TestStream<'a> {
    storage: Vec<Result<&'a [u8], anyhow::Error>>,
}

impl<'a> TestStream<'a> {
    /// Create a new byte stream that only produces bytes successfully.
    pub fn new<V: IntoIterator<Item = T>, T: Into<&'a [u8]>>(items: V) -> Self {
        let mut storage = items
            .into_iter()
            .map(|arr| Ok(arr.into()))
            .collect::<Vec<_>>();
        // So that popping extracts items in the order they were inserted.
        storage.reverse();

        Self { storage }
    }

    /// Create a new byte stream that may also return errors.
    pub fn new_with_err<V: IntoIterator<Item = Result<T, anyhow::Error>>, T: Into<&'a [u8]>>(
        items: V,
    ) -> Self {
        let mut storage = items
            .into_iter()
            .map(|result| result.map(|arr| arr.into()))
            .collect::<Vec<_>>();
        // So that popping extracts items in the order they were inserted.
        storage.reverse();

        Self { storage }
    }
}

impl<'a> tokio::io::AsyncRead for TestStream<'a> {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        match self.get_mut().storage.pop() {
            Some(Ok(byte_array)) => {
                buf.put_slice(byte_array);
                Poll::Ready(Ok(()))
            }
            Some(error) => Poll::Ready(error.map(|_| ()).map_err(std::io::Error::other)),
            None => Poll::Ready(Ok(())),
        }
    }
}

thread_local! {
    /// Base directory where integration tests with sample databases are concluded.
    static REPO_TEST_BASEDIR: LazyCell<PathBuf> = LazyCell::new(|| {
        concat!(env!("CARGO_TARGET_TMPDIR"), "/image_api").into()
    });
}

/// Conveniently prepare a test environment.
///
/// Create a test-specific temporary directory and initialize a SQLite database instance with all
/// migrations applied. Runs a user-provided closure within this environment to run the actual test
/// code.
pub async fn wrap<F>(test_fn: F) -> anyhow::Result<()>
where
    F: AsyncFnOnce(ImageHandler, &std::path::Path) -> anyhow::Result<()>
        + Send
        + Sync
        + UnwindSafe
        + 'static,
{
    let test_dir = REPO_TEST_BASEDIR
        .with(|d| {
            // NOTE(hartan): This is **not** using tokios async version because the closure is sync
            // and I don't feel like refactoring it right now.
            std::fs::create_dir_all(d.as_path())
                .expect("unique test parent directory shouldbe created");
            assert_fs::TempDir::new_in(d.to_path_buf())
        })
        .expect("unique test temp dir should be created");
    tokio::fs::create_dir_all(&test_dir)
        .await
        .expect("unique test directory should be created");

    // Prepare the database instance
    let db_path = test_dir.child("test.db");
    let output = tokio::process::Command::new("diesel")
        .arg("--database-url")
        .arg(db_path.as_os_str())
        .args(["--locked-schema", "migration", "run"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .output()
        .await
        .context("failed to run database migrations for test setup")?;
    if !output.status.success() {
        anyhow::bail!(
            "database migration for test setup encountered an error: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let db = Arc::new(
        db_interaction::connection::DbFacade::new(
            db_path
                .to_str()
                .expect("database path should be valid UTF8 string"),
            1,
        )
        .await
        .context("failed to create DB pool for testing")?,
    );

    let image_store_path = test_dir.join("image_store");
    tokio::fs::create_dir_all(&image_store_path)
        .await
        .expect("image store path should be created");
    let handler = image_api::ImageHandler::new(&image_store_path, db)
        .context("failed to create image handler instance")?;

    std::panic::AssertUnwindSafe(test_fn(handler, image_store_path.as_path()))
        .catch_unwind()
        .await
        .map_err(|e| anyhow::Error::msg(format!("a test panicked: {:#?}", e)))
        .flatten()
        .map_err(|e| {
            let debug_path = test_dir.into_persistent();
            let debug_path = debug_path
                .canonicalize()
                .unwrap_or(debug_path.to_path_buf());
            e.context(format!(
                "test failed, files have been written to {:?}",
                debug_path
            ))
        })
}

/// A file type for checking whether files/directories exist.
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum FileType {
    File,
    Directory,
}

/// Assert whether files or directories exist in a given location.
///
/// Will report both files that aren't expected but still exist as well as files that should exist
/// but aren't present.
pub async fn assert_files<
    P: AsRef<std::path::Path>,
    S: AsRef<std::ffi::OsStr>,
    H: Into<std::collections::HashSet<(FileType, S)>>,
>(
    path: P,
    expect: H,
) -> anyhow::Result<()> {
    let mut entry_iter = tokio::fs::read_dir(&path)
        .await
        .with_context(|| format!("failed to read directory entries in {:?}", path.as_ref()))?;
    let mut expected = expect.into();

    while let Some(entry) = entry_iter
        .next_entry()
        .await
        .with_context(|| format!("failed to read next directory entry in {:?}", path.as_ref()))?
    {
        let name = entry.file_name();
        if (name == "..") || (name == ".") {
            continue;
        }
        dbg!(&name);

        let meta = entry.metadata().await.with_context(|| {
            format!(
                "failed to read metadata of entry {:?} underneath path {:?}",
                entry,
                path.as_ref()
            )
        })?;

        let len_before = expected.len();
        expected.retain(|(expected_type, expected_name)| {
            let result = (expected_name.as_ref() != name)
                || match expected_type {
                    FileType::File => !meta.is_file(),
                    FileType::Directory => !meta.is_dir(),
                };
            dbg!(&name, result);
            result
        });
        if expected.len() == len_before {
            anyhow::bail!(
                "file {:?} was not expected in path {:?}",
                name,
                path.as_ref()
            );
        }
    }

    let leftovers = expected
        .into_iter()
        .map(|(filetype, filename)| match filetype {
            FileType::File => format!("file {:?}", filename.as_ref()),
            FileType::Directory => format!("directory {:?}", filename.as_ref()),
        })
        .collect::<Vec<_>>();
    if leftovers.is_empty() {
        Ok(())
    } else {
        anyhow::bail!(
            "path {:?} DID NOT contain the following expected files: {}",
            path.as_ref(),
            leftovers.join(", ")
        );
    }
}
