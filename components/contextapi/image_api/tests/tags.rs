use anyhow::Context as _;
use image_api::ImageHandlerError;
use image_api::ImageTag;

mod common;

#[test_log::test(tokio::test)]
async fn tags_starts_empty() -> anyhow::Result<()> {
    common::wrap(async |handler, _| {
        let tags = handler
            .list_tags()
            .await
            .expect("image tags should be readable");
        assert!(tags.is_empty());

        Ok(())
    })
    .await
}

#[test_log::test(tokio::test)]
async fn can_add_single_tag() -> anyhow::Result<()> {
    common::wrap(async |handler, _| {
        let new_tag = handler
            .add_tag(ImageTag::new("name", Some("description")))
            .await
            .expect("tag should be added");
        assert_eq!(new_tag.name, "name");

        let tags = handler
            .list_tags()
            .await
            .expect("image tags should be readable");
        assert_eq!(tags.len(), 1);
        assert_eq!(
            &new_tag,
            tags.first().expect("there should be a tag in the database")
        );

        Ok(())
    })
    .await
}

#[test_log::test(tokio::test)]
async fn can_add_multiple_tags() -> anyhow::Result<()> {
    common::wrap(async |handler, _| {
        handler
            .add_tag(ImageTag::new("one", Some("first tag")))
            .await
            .expect("tag should be added");
        let second_tag = handler
            .add_tag(ImageTag::new("two", Some("second tag")))
            .await
            .expect("tag should be added");
        handler
            .add_tag(ImageTag::new("three", Some("third tag")))
            .await
            .expect("tag should be added");

        let tags = handler
            .list_tags()
            .await
            .expect("image tags should be readable");

        assert_eq!(tags.len(), 3);
        assert_eq!(
            &second_tag,
            tags.get(1).expect("there should be a tag in the database")
        );

        Ok(())
    })
    .await
}

#[test_log::test(tokio::test)]
async fn can_modify_existing_tag() -> anyhow::Result<()> {
    common::wrap(async |handler, _| {
        let mut tag = handler
            .add_tag(ImageTag::new("one", Some("first tag")))
            .await
            .expect("tag should be added");

        tag.description = None;

        let updated_tag = handler
            .modify_tag(tag)
            .await
            .expect("image tag should be modified");

        let tags = handler
            .list_tags()
            .await
            .expect("image tags should be readable");

        // Make sure the change also persisted into the database
        assert_eq!(
            &updated_tag,
            tags.first().expect("there should be a tag in the database")
        );
        assert_eq!(updated_tag.name, "one");
        assert!(updated_tag.description.is_none());

        Ok(())
    })
    .await
}

#[test_log::test(tokio::test)]
async fn change_single_tag_only() -> anyhow::Result<()> {
    common::wrap(async |handler, _| {
        handler
            .add_tag(ImageTag::new("one", Some("first tag")))
            .await
            .expect("tag should be added");
        let mut tag = handler
            .add_tag(ImageTag::new("two", Some("second tag")))
            .await
            .expect("tag should be added");

        tag.description = None;

        let updated_tag = handler
            .modify_tag(tag)
            .await
            .context("image tag should be modified")?;

        let tags = handler
            .list_tags()
            .await
            .expect("image tags should be readable");

        // Make sure the change also persisted into the database
        assert_eq!(
            &updated_tag,
            tags.last().expect("there should be a tag in the database")
        );
        assert_eq!(updated_tag.name, "two");
        assert!(updated_tag.description.is_none());

        let first_tag = tags.first().unwrap();
        assert_eq!(first_tag.name, "one");
        assert!(first_tag.description.is_some());

        Ok(())
    })
    .await
}

#[test_log::test(tokio::test)]
async fn cannot_update_nonexistent_tag() -> anyhow::Result<()> {
    common::wrap(async |handler, _| {
        let fresh_tag = ImageTag::new("one", Some("first tag"));
        let error = handler.modify_tag(fresh_tag).await;

        assert!(error.is_err());

        Ok(())
    })
    .await
}

#[test_log::test(tokio::test)]
async fn cannot_insert_existent_tag() -> anyhow::Result<()> {
    common::wrap(async |handler, _| {
        let tag = handler
            .add_tag(ImageTag::new("one", Some("first tag")))
            .await
            .expect("tag should be added");

        let result = handler.add_tag(tag).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(ImageHandlerError::MetadataError)));

        Ok(())
    })
    .await
}

#[test_log::test(tokio::test)]
async fn can_remove_tags() -> anyhow::Result<()> {
    common::wrap(async |handler, _| {
        let tag = handler
            .add_tag(ImageTag::new("one", Some("first tag")))
            .await
            .expect("tag should be added");
        handler
            .remove_tag(tag)
            .await
            .expect("tag should be removed");

        let tag_list = handler.list_tags().await.expect("tags should be listed");

        assert!(tag_list.is_empty());

        Ok(())
    })
    .await
}

#[test_log::test(tokio::test)]
async fn can_remove_single_tag_only() -> anyhow::Result<()> {
    common::wrap(async |handler, _| {
        let tag = handler
            .add_tag(ImageTag::new("one", Some("first tag")))
            .await
            .expect("tag should be added");
        let second_tag = handler
            .add_tag(ImageTag::new("two", Some("second tag")))
            .await
            .expect("tag should be added");

        handler
            .remove_tag(tag)
            .await
            .expect("tag should be removed");

        let tag_list = handler.list_tags().await.expect("tags should be listed");
        assert_eq!(tag_list.len(), 1);

        let remaining_tag = tag_list.first().expect("tag list must have single entry");
        assert_eq!(remaining_tag, &second_tag);

        Ok(())
    })
    .await
}
