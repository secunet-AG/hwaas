mod common;

use anyhow::Context as _;
use common::TestStream;
use image_api::{ImageHandler, ImageMetadata, ImageTag};

async fn add_image(
    handler: &ImageHandler,
    name: &str,
    content: &str,
) -> anyhow::Result<ImageMetadata> {
    let stream = TestStream::new([content.as_bytes()]);
    let metadata = image_api::ExtraImageStoreData {
        user_file_name: name.to_string(),
        compression: image_api::Compression::None,
    };

    handler
        .add_image(stream, metadata)
        .await
        .context("failed to add new image")
}

async fn add_tag(handler: &ImageHandler, name: &str, content: &str) -> anyhow::Result<ImageTag> {
    let tag = ImageTag::new(name, Some(content));

    handler.add_tag(tag).await.context("failed to add new tag")
}

#[test_log::test(tokio::test)]
async fn new_image_has_no_tags() -> anyhow::Result<()> {
    common::wrap(async |handler, _| {
        let image = add_image(&handler, "one", "bytes for image one").await?;
        add_tag(&handler, "tag_a", "sample tag A").await?;
        add_tag(&handler, "tag_b", "sample tag B").await?;

        let images = handler.list_image_metadatas().await.unwrap();
        let tags = handler.list_tags().await.unwrap();

        assert_eq!(images.len(), 1);
        assert_eq!(tags.len(), 2);

        let tags_on_image = handler
            .get_image_metadata_by_hash(&image.sha256)
            .await
            .context("requested image should exist")?
            .tags;
        assert!(tags_on_image.is_empty());

        Ok(())
    })
    .await
}

#[test_log::test(tokio::test)]
async fn can_add_tags_to_image() -> anyhow::Result<()> {
    common::wrap(async |handler, _| {
        let image = add_image(&handler, "one", "bytes for image one").await?;
        let tag_a = add_tag(&handler, "tag_a", "sample tag A").await?;
        let tag_b = add_tag(&handler, "tag_b", "sample tag B").await?;

        let num_added = handler
            .add_tags_to_image(["tag_a".to_string(), "tag_b".to_string()], &image.sha256)
            .await
            .context("tags should be attached to image")?;
        assert_eq!(num_added, 2);

        let mut image_tags = handler
            .get_image_metadata_by_hash(&image.sha256)
            .await
            .context("requested image should exist")?
            .tags;
        image_tags.sort_by(|lhs, rhs| lhs.name.cmp(&rhs.name));
        let mut tag_iter = image_tags.into_iter();

        assert_eq!(tag_iter.next().unwrap(), tag_a);
        assert_eq!(tag_iter.next().unwrap(), tag_b);
        assert!(tag_iter.next().is_none());

        Ok(())
    })
    .await
}

#[test_log::test(tokio::test)]
async fn can_remove_tags_from_image() -> anyhow::Result<()> {
    common::wrap(async |handler, _| {
        let image = add_image(&handler, "one", "bytes for image one").await?;
        let _tag_a = add_tag(&handler, "tag_a", "sample tag A").await?;
        let tag_b = add_tag(&handler, "tag_b", "sample tag B").await?;

        let num_added = handler
            .add_tags_to_image(["tag_a".to_string(), "tag_b".to_string()], &image.sha256)
            .await
            .context("tags should be attached to image")?;
        assert_eq!(num_added, 2);
        let num_removed = handler
            .remove_tags_from_image(["tag_a".to_string()], &image.sha256)
            .await
            .context("tags should be removed from image")?;
        assert_eq!(num_removed, 1);

        let mut image_tags = handler
            .get_image_metadata_by_hash(&image.sha256)
            .await
            .context("requested image should exist")?
            .tags;
        image_tags.sort_by(|lhs, rhs| lhs.name.cmp(&rhs.name));
        let mut tag_iter = image_tags.into_iter();

        assert_eq!(tag_iter.next().unwrap(), tag_b);
        assert!(tag_iter.next().is_none());

        Ok(())
    })
    .await
}
