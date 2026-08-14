// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

mod common;

use image_api::MaintenanceOperations;

#[test_log::test(tokio::test)]
async fn nothing_to_do() -> anyhow::Result<()> {
    common::wrap(async |handler, _| {
        let result = handler.maintenance(MaintenanceOperations::all()).await;
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

        let result = handler.maintenance(MaintenanceOperations::all()).await;

        assert!(result.is_ok());
        assert!(!random_file.exists());

        Ok(())
    })
    .await
}
