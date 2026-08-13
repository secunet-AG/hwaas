// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::inventory::SwitchMapping;
use crate::inventory::backend::InventoryBackendApi;
use async_trait::async_trait;
use std::path::PathBuf;
use system_error::Error as SysError;

pub struct PushInventoryFileBackend {
    _file_path: PathBuf,
    mapping: SwitchMapping,
}

impl PushInventoryFileBackend {
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            _file_path: file_path,
            mapping: SwitchMapping::default(),
        }
    }
}

#[async_trait]
impl InventoryBackendApi for PushInventoryFileBackend {
    async fn get_switch_mapping(&self) -> Result<SwitchMapping, SysError> {
        Ok(self.mapping.clone())
    }
}
