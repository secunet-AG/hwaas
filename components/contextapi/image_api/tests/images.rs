mod common;

use anyhow::Context as _;
use common::{assert_files, FileType, TestStream};
use image_api::ExtraImageStoreData;

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

fn image_meta(name: &'static str) -> ExtraImageStoreData {
    ExtraImageStoreData {
        user_file_name: name.to_string(),
        compression: image_api::Compression::None,
    }
}

#[test_log::test(tokio::test)]
async fn can_add_single_image() -> anyhow::Result<()> {
    common::wrap(async |handler, _| {
        let sample_stream = TestStream::new(["a bunch of bytes".as_bytes()]);

        let new_image = handler
            .add_image(sample_stream, image_meta("my cool image.bmrimg"))
            .await
            .context("image should be added")?;
        assert_eq!(new_image.file_name, "my cool image.bmrimg");

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
                image_meta("one"),
            )
            .await
            .context("image should be added")?;
        let second_image = handler
            .add_image(
                TestStream::new(["bytes for image two".as_bytes()]),
                image_meta("two"),
            )
            .await
            .context("image should be added")?;
        handler
            .add_image(
                TestStream::new(["bytes for image three".as_bytes()]),
                image_meta("three"),
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
                image_meta("one"),
            )
            .await
            .context("image should be added")?;

        image.file_name = "two?".into();
        image.architecture = Some("risc-v".into());

        let updated_image = handler
            .modify_image_metadata(image)
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
        assert_eq!(updated_image.file_name, "two?");
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

                handler.modify_image_metadata(image).await
            }};
        }

        handler
            .add_image(
                TestStream::new(["bytes for sample image".as_bytes()]),
                image_meta("one"),
            )
            .await
            .expect("image should be added");

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
            .fold((vec![], vec![], vec![]), |mut acc, obj| {
                acc.0.push(obj.sha256);
                acc.1.push(obj.size_bytes);
                acc.2.push(obj.created_utc);
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
                image_meta("one"),
            )
            .await
            .expect("image should be added");
        let mut image = handler
            .add_image(
                TestStream::new(["moaaar bytes for another image".as_bytes()]),
                image_meta("two"),
            )
            .await
            .expect("image should be added");

        image.file_name = "three".into();
        image.architecture = Some("bla".into());

        let updated_image = handler
            .modify_image_metadata(image)
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
        assert_eq!(updated_image.file_name, "three");
        assert!(updated_image.architecture.is_some_and(|d| d == "bla"));

        let first_image = images.first().unwrap();
        assert_eq!(first_image.file_name, "one");
        assert!(first_image.architecture.is_none());

        Ok(())
    })
    .await
}

// NOTE: This behavior is dictated by the initial Web API, where inserting an existing image a
// second time was not considered an error. Instead, certain metadata is in fact updated.
#[test_log::test(tokio::test)]
async fn can_insert_existent_image() -> anyhow::Result<()> {
    common::wrap(async |handler, _| {
        let metadata1 = handler
            .add_image(
                TestStream::new(["bytes for image one".as_bytes()]),
                image_meta("one"),
            )
            .await
            .expect("tag should be added");

        let maybe_metadata2 = handler
            .add_image(
                TestStream::new(["bytes for image one".as_bytes()]),
                image_meta("two"),
            )
            .await;
        assert!(maybe_metadata2.is_ok());
        let metadata2 = maybe_metadata2.unwrap();

        assert_eq!(metadata1.sha256, metadata2.sha256);
        // NOTE: Upload timestamp and filename is updated to signal that this image is still of
        // interest.
        assert_ne!(metadata1.created_utc, metadata2.created_utc);
        assert_ne!(metadata1.file_name, metadata2.file_name);

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
                image_meta("one"),
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
                image_meta("one"),
            )
            .await
            .expect("image metadata should be added");
        let second_image = handler
            .add_image(
                TestStream::new(["bytes for image two".as_bytes()]),
                image_meta("two"),
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
                image_meta("one"),
            )
            .await
            .expect("image metadata should be added");
        let second_image = handler
            .add_image(
                TestStream::new(["bytes for image two".as_bytes()]),
                image_meta("two"),
            )
            .await
            .expect("image metadata should be added");

        let first_image_path = image_store.join(first_image.sha256);
        assert!(first_image_path.exists());
        let image_content = tokio::fs::read_to_string(first_image_path)
            .await
            .context("failed to read image content from disk")?;
        assert_eq!(image_content, "bytes for image one");

        assert!(image_store.join(second_image.sha256).exists());

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
                image_meta("invalid"),
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
