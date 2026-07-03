mod common;

use anyhow::Context as _;

#[test_log::test(tokio::test)]
async fn can_migrate_nothing() -> anyhow::Result<()> {
    common::wrap(async |mut handler, _| {
        let result = handler.migrate_legacy_images().await;
        assert!(result.is_ok());

        Ok(())
    })
    .await
}

#[test_log::test(tokio::test)]
async fn migrate_single_image() -> anyhow::Result<()> {
    common::wrap(async |mut handler, image_store| {
        let image_hash = "68360331653ff7a1ffbad51d9f5157eae1ce7b1219650309fb6034b985ba3857";
        let legacy_image_path = image_store.join(image_hash);
        tokio::fs::write(&legacy_image_path, "This image has content, too")
            .await
            .unwrap();
        tokio::fs::write(
            &legacy_image_path.with_added_extension("txt"),
            "image_name.iso",
        )
        .await
        .unwrap();

        let result = handler.migrate_legacy_images().await;

        assert!(result.is_ok());

        let images = handler
            .list_images()
            .await
            .context("image metadata should be readable")?;
        assert_eq!(images.len(), 1);
        let the_image = images.first().unwrap();
        assert_eq!(the_image.file_name, "image_name.iso");
        assert_eq!(the_image.size_bytes, 27);
        assert_eq!(the_image.sha256, image_hash);

        // Image file is cleared up after migration
        assert!(!legacy_image_path.exists());

        Ok(())
    })
    .await
}

#[test_log::test(tokio::test)]
async fn dont_migrate_random_files() -> anyhow::Result<()> {
    common::wrap(async |mut handler, image_store| {
        let random_file = image_store.join("random-file");
        tokio::fs::write(random_file, "this content is ignored, right?")
            .await
            .unwrap();

        let result = handler.migrate_legacy_images().await;
        assert!(result.is_ok());

        let images = handler
            .list_images()
            .await
            .context("image metadata should be readable")?;
        assert!(images.is_empty());

        Ok(())
    })
    .await
}
