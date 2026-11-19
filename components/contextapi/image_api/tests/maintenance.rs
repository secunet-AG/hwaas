// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

mod common;

use anyhow::Context as _;
use common::TestStream;
use image_api::{ExtraImageStoreData, GarbagePolicy, MaintenanceOperations};

#[test_log::test(tokio::test)]
async fn nothing_to_do() -> anyhow::Result<()> {
    common::wrap(async |handler, _| {
        let result = handler
            .maintenance(vec![MaintenanceOperations::PruneUnusedFiles])
            .await;
        assert!(result.is_ok());

        Ok(())
    })
    .await
}

#[test_log::test(tokio::test)]
async fn clean_up_single_file() -> anyhow::Result<()> {
    common::wrap(async |handler, image_store| {
        let random_file = image_store.join("random-file.txt");
        tokio::fs::write(&random_file, "This file has content")
            .await
            .unwrap();
        assert!(&random_file.exists());

        let result = handler
            .maintenance(vec![MaintenanceOperations::PruneUnusedFiles])
            .await;

        assert!(result.is_ok());
        assert!(!random_file.exists());

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
async fn gc_image_by_age() -> anyhow::Result<()> {
    common::wrap(async |handler, _| {
        let older_image = handler
            .add_image(
                TestStream::new(["content".as_bytes()]),
                image_meta("my cool image.bmrimg"),
            )
            .await
            .context("image should be added")?;
        // NOTE(hartan): I'm aware that it's not a great experience to make tests sleep like this.
        // But in this case we cannot work around the "limitations" of monotone clocks since we're
        // using `SystemTime` under the hood to record the timestamps of images during uploads. That
        // means we cannot artificially slow down or speed up time during tests. Writing the test
        // like this gives the host running the code ~3 seconds to get from uploading the second
        // image to asserting the duration during garbage collection to pass the test. This should
        // hopefully be enough even for slow GHA runners.
        tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;
        let newer_image = handler
            .add_image(
                TestStream::new(["other_content".as_bytes()]),
                image_meta("my other cool image.bmrimg"),
            )
            .await
            .context("image should be added")?;

        let images = handler
            .list_image_metadatas()
            .await
            .context("image metadata should be readable")?;
        assert_eq!(images.len(), 2);

        let result = handler
            .maintenance(vec![MaintenanceOperations::GarbageCollectImages(
                GarbagePolicy::OlderThan(chrono::DateTime::<chrono::Utc>::from(
                    older_image.created + std::time::Duration::from_secs(1),
                )),
            )])
            .await;
        assert!(result.is_ok());

        let images = handler
            .list_image_metadatas()
            .await
            .context("image metadata should be readable")?;
        assert_eq!(images.len(), 1);
        assert_eq!(images.first().unwrap().sha256, newer_image.sha256);

        Ok(())
    })
    .await
}

#[test_log::test(tokio::test)]
async fn gc_image_by_size() -> anyhow::Result<()> {
    common::wrap(async |handler, _| {
        let smaller_image = handler
            .add_image(
                TestStream::new(["content".as_bytes()]),
                image_meta("my cool image.bmrimg"),
            )
            .await
            .context("image should be added")?;
        let _ = handler
            .add_image(
                TestStream::new(["other_content".as_bytes()]),
                image_meta("my other cool image.bmrimg"),
            )
            .await
            .context("image should be added")?;

        let images = handler
            .list_image_metadatas()
            .await
            .context("image metadata should be readable")?;
        assert_eq!(images.len(), 2);

        let result = handler
            .maintenance(vec![MaintenanceOperations::GarbageCollectImages(
                GarbagePolicy::LargerThanBytes(10),
            )])
            .await;
        assert!(result.is_ok());

        let images = handler
            .list_image_metadatas()
            .await
            .context("image metadata should be readable")?;
        assert_eq!(images.len(), 1);
        assert_eq!(images.first().unwrap().sha256, smaller_image.sha256);

        Ok(())
    })
    .await
}
