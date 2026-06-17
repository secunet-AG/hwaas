use anyhow::Context as _;
use assert_fs::fixture::PathChild as _;
use futures::FutureExt as _;
use image_api::ImageHandler;
use std::cell::LazyCell;
use std::panic::UnwindSafe;
use std::path::PathBuf;
use std::sync::Arc;

thread_local! {
    /// Base directory where integration tests with sample databases are concluded are concluded.
    static REPO_TEST_BASEDIR: LazyCell<PathBuf> = LazyCell::new(|| {
        concat!(env!("CARGO_TARGET_TMPDIR"), "/image_api").into()
    });
}

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
