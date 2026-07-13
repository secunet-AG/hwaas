// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::backend::InventoryBackendApi;
use crate::SwitchMapping;
use async_trait::async_trait;
use libc::{EBADMSG, ENOENT};
use std::path::PathBuf;
use system_error::Error as SysError;
use tokio::fs;
use tracing::error;

pub struct PullInventoryFileBackend {
    file_path: PathBuf,
}

impl PullInventoryFileBackend {
    pub fn new(file_path: PathBuf) -> Self {
        Self { file_path }
    }
}

#[async_trait]
impl InventoryBackendApi for PullInventoryFileBackend {
    async fn get_switch_mapping(&self) -> Result<SwitchMapping, SysError> {
        let contents = fs::read_to_string(&self.file_path).await.map_err(|e| {
            error!(error = %e, "Could not read inventory file");
            SysError::from_raw_os_error(ENOENT)
        })?;
        Ok(
            serde_json::from_str::<SwitchMapping>(contents.as_str()).map_err(|e| {
                error!(error = %e, "Could not parse inventory file");
                SysError::from_raw_os_error(EBADMSG)
            })?,
        )
    }

    async fn update(&mut self) {
        // nothing to do - the file is read at the next `get_switch_mapping` is called
    }
}
