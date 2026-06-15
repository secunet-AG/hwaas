use anyhow::Context as _;
use image_api;
use image_api::ImageTag;
use std::path::PathBuf;
use std::sync::Arc;
use tokio;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let db = Arc::new(db_interaction::connection::DbFacade::new("/var/home/hartan/repos/gitlab.cyberus-technology.de/cyberus/cidoka/hwaas/hwaas/components/contextapi/development.db", 1).await.context("failed to create DB pool")?);
    let handler = image_api::ImageHandler::new(PathBuf::from("./query_tags_example_store"), db)
        .context("failed to create image handler instance")?;

    handler
        .add_tag(ImageTag::new(
            "hallo",
            Some("dies ist nur ein Test, keine Panik!"),
        ))
        .await
        .expect("failed to insert new tag");
    dbg!(handler
        .list_images()
        .await
        .expect("failed to list container images"));
    dbg!(handler
        .list_tags()
        .await
        .expect("failed to list container tags"));
    Ok(())
}
