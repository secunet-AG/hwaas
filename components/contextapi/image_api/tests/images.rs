mod common;

use anyhow::Context as _;
use common::{assert_files, FileType, TestStream};

#[test_log::test(tokio::test)]
async fn images_starts_empty() -> anyhow::Result<()> {
    common::wrap(async |handler, _| {
        let images = handler
            .list_images()
            .await
            .context("image metadatas should be readable")?;
        assert!(images.is_empty());

        Ok(())
    })
    .await
}

#[test_log::test(tokio::test)]
async fn can_add_single_image() -> anyhow::Result<()> {
    common::wrap(async |handler, _| {
        let sample_stream = TestStream::new(["a bunch of bytes".as_bytes()]);

        let new_image = handler
            .add_image(sample_stream, "my cool image.bmrimg".to_string())
            .await
            .context("image should be added")?;
        assert_eq!(new_image.upload_name, "my cool image.bmrimg");

        let images = handler
            .list_images()
            .await
            .context("image metadata should be readable")?;
        assert_eq!(images.len(), 1);
        assert_eq!(
            &new_image,
            images
                .first()
                .context("there should be a tag in the database")?
        );

        Ok(())
    })
    .await
}

#[test_log::test(tokio::test)]
async fn can_add_multiple_images() -> anyhow::Result<()> {
    common::wrap(async |handler, _| {
        handler
            .add_image(
                TestStream::new(["bytes for image one".as_bytes()]),
                "one".to_string(),
            )
            .await
            .context("image should be added")?;
        let second_image = handler
            .add_image(
                TestStream::new(["bytes for image two".as_bytes()]),
                "two".to_string(),
            )
            .await
            .context("image should be added")?;
        handler
            .add_image(
                TestStream::new(["bytes for image three".as_bytes()]),
                "three".to_string(),
            )
            .await
            .context("image should be added")?;

        let images = handler
            .list_images()
            .await
            .context("image metadata should be readable")?;

        assert_eq!(images.len(), 3);
        assert_eq!(
            &second_image,
            images
                .get(1)
                .expect("there should be image metadata in the database")
        );

        Ok(())
    })
    .await
}

#[test_log::test(tokio::test)]
async fn can_modify_existing_image_partially() -> anyhow::Result<()> {
    common::wrap(async |handler, _| {
        let mut image = handler
            .add_image(
                TestStream::new(["bytes for image one".as_bytes()]),
                "one".to_string(),
            )
            .await
            .context("image should be added")?;

        image.upload_name = "two?".into();
        image.architecture = Some("risc-v".into());

        let updated_image = handler
            .modify_image(image)
            .await
            .context("image metadata should be modified")?;

        let images = handler
            .list_images()
            .await
            .context("image metadata should be readable")?;

        // Make sure the change also persisted into the database
        assert_eq!(
            &updated_image,
            images
                .first()
                .expect("there should be image metadata in the database")
        );
        assert_eq!(updated_image.upload_name, "two?");
        assert!(updated_image.architecture.is_some_and(|s| s == "risc-v"));

        Ok(())
    })
    .await
}

#[test_log::test(tokio::test)]
async fn must_not_modify_certain_image_fields() -> anyhow::Result<()> {
    common::wrap(async |handler, _| {
        macro_rules! quick_modify {
            ($field:ident, $value:expr) => {{
                let mut image = handler
                    .list_images()
                    .await
                    .expect("image should be added")
                    .pop()
                    .expect("there should be one image to list");

                image.$field = $value;

                handler.modify_image(image).await
            }};
        }

        handler
            .add_image(
                TestStream::new(["bytes for sample image".as_bytes()]),
                "one".to_string(),
            )
            .await
            .expect("image should be added");

        assert!(quick_modify!(file_name, "invalid".to_string()).is_err());
        let sample_hash = image_api::sha256hash::Sha256Hash::new(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
        )
        .unwrap();
        assert!(quick_modify!(sha256, sample_hash.0).is_err());
        assert!(quick_modify!(size_bytes, 12312i64).is_err());
        assert!(quick_modify!(created_utc, chrono::DateTime::<chrono::Utc>::default()).is_err());

        let images = handler
            .list_images()
            .await
            .context("image metadata should be readable")?;

        let fields = images
            .into_iter()
            .fold((vec![], vec![], vec![], vec![]), |mut acc, obj| {
                acc.0.push(obj.file_name);
                acc.1.push(obj.sha256);
                acc.2.push(obj.size_bytes);
                acc.3.push(obj.created_utc);
                acc
            });
        dbg!(&fields);

        // All the non-mutable fields should have equal contents.
        assert!(fields
            .0
            .iter()
            .all(|elem| elem == fields.0.first().unwrap()));
        assert!(fields
            .1
            .iter()
            .all(|elem| elem == fields.1.first().unwrap()));
        assert!(fields
            .2
            .iter()
            .all(|elem| elem == fields.2.first().unwrap()));
        assert!(fields
            .3
            .iter()
            .all(|elem| elem == fields.3.first().unwrap()));
        Ok(())
    })
    .await
}

#[test_log::test(tokio::test)]
async fn change_single_image_only() -> anyhow::Result<()> {
    common::wrap(async |handler, _| {
        handler
            .add_image(
                TestStream::new(["bytes for sample image".as_bytes()]),
                "one".to_string(),
            )
            .await
            .expect("image should be added");
        let mut image = handler
            .add_image(
                TestStream::new(["moaaar bytes for another image".as_bytes()]),
                "two".to_string(),
            )
            .await
            .expect("image should be added");

        image.upload_name = "three".into();
        image.architecture = Some("bla".into());

        let updated_image = handler
            .modify_image(image)
            .await
            .context("image metadata should be modified")?;

        let images = handler
            .list_images()
            .await
            .expect("image metadata should be readable");

        // Make sure the change also persisted into the database
        assert_eq!(
            &updated_image,
            images
                .last()
                .expect("there should be a tag in the database")
        );
        assert_eq!(updated_image.upload_name, "three");
        assert!(updated_image.architecture.is_some_and(|d| d == "bla"));

        let first_image = images.first().unwrap();
        assert_eq!(first_image.upload_name, "one");
        assert!(first_image.architecture.is_none());

        Ok(())
    })
    .await
}

// NOTE: This behavior is dictated by the initial Web API, where inserting an existing image a
// second time was not considered an error.
#[test_log::test(tokio::test)]
async fn can_insert_existent_image() -> anyhow::Result<()> {
    common::wrap(async |handler, _| {
        let metadata1 = handler
            .add_image(
                TestStream::new(["bytes for image one".as_bytes()]),
                "one".to_string(),
            )
            .await
            .expect("tag should be added");

        let maybe_metadata2 = handler
            .add_image(
                TestStream::new(["bytes for image one".as_bytes()]),
                "two".to_string(),
            )
            .await;
        assert!(maybe_metadata2.is_ok());
        let metadata2 = maybe_metadata2.unwrap();

        assert_eq!(metadata1.sha256, metadata2.sha256);
        assert_eq!(metadata1.upload_name, metadata2.upload_name);
        // NOTE: Upload timestamp is updated to signal that this image is still of interest.
        assert_ne!(metadata1.created_utc, metadata2.created_utc);

        Ok(())
    })
    .await
}

#[test_log::test(tokio::test)]
async fn can_remove_images() -> anyhow::Result<()> {
    common::wrap(async |handler, _| {
        let image = handler
            .add_image(
                TestStream::new(["bytes for image one".as_bytes()]),
                "one".to_string(),
            )
            .await
            .expect("image metadata should be added");
        handler
            .remove_image(image)
            .await
            .context("image metadata should be removed")?;

        let image_list = handler
            .list_images()
            .await
            .expect("image metadata should be listed");

        assert!(image_list.is_empty());

        Ok(())
    })
    .await
}

#[test_log::test(tokio::test)]
async fn can_remove_single_image_only() -> anyhow::Result<()> {
    common::wrap(async |handler, _| {
        let image = handler
            .add_image(
                TestStream::new(["bytes for image one".as_bytes()]),
                "one".to_string(),
            )
            .await
            .expect("image metadata should be added");
        let second_image = handler
            .add_image(
                TestStream::new(["bytes for image two".as_bytes()]),
                "two".to_string(),
            )
            .await
            .expect("image metadata should be added");

        handler
            .remove_image(image)
            .await
            .expect("image metadata should be removed");

        let image_list = handler
            .list_images()
            .await
            .expect("image metadata should be listed");
        assert_eq!(image_list.len(), 1);

        let remaining_image = image_list
            .first()
            .expect("image metadata list must have single entry");
        assert_eq!(remaining_image, &second_image);

        Ok(())
    })
    .await
}

#[test_log::test(tokio::test)]
async fn images_are_written_to_disk() -> anyhow::Result<()> {
    common::wrap(async |handler, image_store| {
        let first_image = handler
            .add_image(
                TestStream::new(["bytes for image one".as_bytes()]),
                "one".to_string(),
            )
            .await
            .expect("image metadata should be added");
        let second_image = handler
            .add_image(
                TestStream::new(["bytes for image two".as_bytes()]),
                "two".to_string(),
            )
            .await
            .expect("image metadata should be added");

        let first_image_path = image_store.join(first_image.file_name);
        assert!(first_image_path.exists());
        let image_content = tokio::fs::read_to_string(first_image_path)
            .await
            .context("failed to read image content from disk")?;
        assert_eq!(image_content, "bytes for image one");

        assert!(image_store.join(second_image.file_name).exists());

        Ok(())
    })
    .await
}

#[test_log::test(tokio::test)]
async fn invalid_image_leaves_no_trace_in_fs() -> anyhow::Result<()> {
    common::wrap(async |handler, image_store| {
        let result = handler
            .add_image(
                TestStream::new_with_err([
                    Ok("bytes for image one".as_bytes()),
                    Err(anyhow::Error::msg("Sorry, this won't work")),
                ]),
                "invalid".to_string(),
            )
            .await;
        assert!(result.is_err());

        let expected = [(FileType::Directory, "uploads")];
        assert_files(image_store, expected).await.unwrap();

        // The uploads folder must be empty
        assert_files::<_, std::ffi::OsString, _>(image_store.join("uploads"), [])
            .await
            .unwrap();

        Ok(())
    })
    .await
}
