// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

mod common;

use anyhow::Context as _;
use common::TestStream;
use image_api::architectures::Architecture;
use image_api::sha256hash::Sha256Hash;
use image_api::{ImageHandler, ImageHandlerError, ImageMetadata, ImageMetadataUpdate, ImageTag};

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

#[test_log::test(tokio::test)]
async fn list_images_with_files_left_joins_files_with_metadata() -> anyhow::Result<()> {
    common::wrap(async |handler, store_path| {
        let tagged = add_image(&handler, "tagged", "bytes for tagged image").await?;
        let untagged = add_image(&handler, "untagged", "bytes for untagged image").await?;
        let missing = add_image(&handler, "missing", "bytes for missing image").await?;

        // Add two tags to the DB
        add_tag(&handler, "tag_a", "sample tag A").await?;
        add_tag(&handler, "tag_b", "sample tag B").await?;

        // Connect these tags to an image
        handler
            .add_tags_to_image(["tag_a".to_string(), "tag_b".to_string()], &tagged.sha256)
            .await?;

        // Connect these tags to an image that we will shortly delete
        handler
            .add_tags_to_image(["tag_a".to_string()], &missing.sha256)
            .await?;

        // We now drop the "missing" image, this is because we want to make sure
        // we aren't pulling in metadata in our list for images that are not present
        // in the filesystem
        tokio::fs::remove_file(store_path.join(&missing.sha256.0)).await?;

        // A file on disk without metadata
        let unknown_hash = "a".repeat(64);
        tokio::fs::write(store_path.join(&unknown_hash), "unknown image").await?;

        // Just a junk file, but we will still expect this to be present, as
        // we are "joining" on the filesystem.
        tokio::fs::write(store_path.join("not-an-image.txt"), "junk").await?;

        let images = handler
            .list_images_with_files()
            .await
            .context("images should be listable")?;

        let mut hashes = images
            .iter()
            .map(|(hash, _)| hash.0.as_str())
            .collect::<Vec<_>>();

        hashes.sort();

        // We expect to see both the tagged and untagged image, as well as the junk hash.
        //
        // We do not want to see the missing image, as we only are "joining" on images that
        // are present in the filesystem.
        //
        // We need to sort to assert equality, could also be a set.
        let mut expected = [
            tagged.sha256.0.as_str(),
            untagged.sha256.0.as_str(),
            unknown_hash.as_str(),
        ];
        expected.sort();

        assert_eq!(hashes, expected);

        // A quick function to test if an image is present
        let find = |hash: &str| {
            images
                .iter()
                .find(|(h, _)| h.0 == hash)
                .map(|(_, meta)| meta.as_ref())
                .expect("image should be listed")
        };

        // Our tagged image will have metadata
        let meta = find(&tagged.sha256.0).context("tagged image should have metadata")?;
        let mut names = meta
            .tags
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>();

        names.sort();

        // The two tags we added earlier
        assert_eq!(names, ["tag_a", "tag_b"]);

        let meta = find(&untagged.sha256.0).context("untagged image should have metadata")?;
        assert!(meta.tags.is_empty());

        assert!(find(&unknown_hash).is_none());

        Ok(())
    })
    .await
}

/// Regression test as we were picking up files containing the hash + .txt, they files
/// had the same inode backing so there was not any duplication, but it caused a lot of
/// noise when trying to make a nice API for viewing available images.
#[test_log::test(tokio::test)]
async fn list_images_with_files_ignores_files_merely_containing_a_hash() -> anyhow::Result<()> {
    common::wrap(async |handler, store_path| {
        let image = add_image(&handler, "image", "bytes for image").await?;

        // Hard links of the image whose names contain, but aren't exactly, the hash
        let image_path = store_path.join(&image.sha256.0);
        for name in [
            format!("{}.txt", image.sha256.0),
            format!("prefix-{}", image.sha256.0),
        ] {
            tokio::fs::hard_link(&image_path, store_path.join(name)).await?;
        }

        let images = handler
            .list_images_with_files()
            .await
            .context("images should be listable")?;

        let hashes = images
            .iter()
            .map(|(hash, _)| hash.0.as_str())
            .collect::<Vec<_>>();

        // We should not see the other hard links to this image
        assert_eq!(hashes, [image.sha256.0.as_str()]);

        Ok(())
    })
    .await
}

fn tag_names(metadata: &ImageMetadata) -> Vec<String> {
    let mut names = metadata
        .tags
        .iter()
        .map(|t| t.name.clone())
        .collect::<Vec<_>>();
    names.sort();
    names
}

#[test_log::test(tokio::test)]
async fn update_replaces_tags_and_creates_unknown_ones() -> anyhow::Result<()> {
    common::wrap(async |handler, _| {
        let image = add_image(&handler, "one", "bytes for image one").await?;
        let tag_a = add_tag(&handler, "tag_a", "sample tag A").await?;
        add_tag(&handler, "tag_b", "sample tag B").await?;
        handler
            .add_tags_to_image(["tag_b".to_string()], &image.sha256)
            .await?;

        let update = ImageMetadataUpdate {
            tags: Some(vec!["tag_a".into(), "tag_new".into(), "tag_new".into()]),
            ..Default::default()
        };
        let updated = handler
            .update_image_metadata(&image.sha256, update)
            .await
            .context("metadata update should succeed")?;

        assert_eq!(tag_names(&updated), ["tag_a", "tag_new"]);
        // Existing tags keep their description, new ones get none.
        assert!(updated.tags.contains(&tag_a));
        assert!(
            updated
                .tags
                .contains(&ImageTag::new("tag_new", None::<String>))
        );
        // Detached tags still exist globally.
        assert_eq!(handler.list_tags().await?.len(), 3);

        Ok(())
    })
    .await
}

#[test_log::test(tokio::test)]
async fn update_with_empty_tag_list_clears_tags() -> anyhow::Result<()> {
    common::wrap(async |handler, _| {
        let image = add_image(&handler, "one", "bytes for image one").await?;
        add_tag(&handler, "tag_a", "sample tag A").await?;
        handler
            .add_tags_to_image(["tag_a".to_string()], &image.sha256)
            .await?;

        let update = ImageMetadataUpdate {
            tags: Some(vec![]),
            ..Default::default()
        };
        let updated = handler.update_image_metadata(&image.sha256, update).await?;
        assert!(updated.tags.is_empty());

        Ok(())
    })
    .await
}

#[test_log::test(tokio::test)]
async fn update_only_touches_given_fields() -> anyhow::Result<()> {
    common::wrap(async |handler, _| {
        let image = add_image(&handler, "one", "bytes for image one").await?;
        add_tag(&handler, "tag_a", "sample tag A").await?;
        handler
            .add_tags_to_image(["tag_a".to_string()], &image.sha256)
            .await?;

        let update = ImageMetadataUpdate {
            architecture: Some(Some(Architecture::Riscv64)),
            ..Default::default()
        };
        let updated = handler.update_image_metadata(&image.sha256, update).await?;
        assert_eq!(updated.architecture, Some(Architecture::Riscv64));
        assert_eq!(updated.file_name, "one");
        assert_eq!(tag_names(&updated), ["tag_a"]);

        let update = ImageMetadataUpdate {
            file_name: Some("renamed".into()),
            architecture: Some(None),
            ..Default::default()
        };
        let updated = handler.update_image_metadata(&image.sha256, update).await?;
        assert_eq!(updated.architecture, None);
        assert_eq!(updated.file_name, "renamed");
        assert_eq!(tag_names(&updated), ["tag_a"]);

        Ok(())
    })
    .await
}

#[test_log::test(tokio::test)]
async fn update_of_unknown_image_is_not_found() -> anyhow::Result<()> {
    common::wrap(async |handler, _| {
        let unknown = Sha256Hash::new("0".repeat(64)).expect("valid hash");
        let update = ImageMetadataUpdate {
            tags: Some(vec!["tag_a".into()]),
            ..Default::default()
        };
        let result = handler.update_image_metadata(&unknown, update).await;
        assert!(matches!(result, Err(ImageHandlerError::NotFound(_))));
        // The transaction must not leave a created tag behind.
        assert!(handler.list_tags().await?.is_empty());

        Ok(())
    })
    .await
}
