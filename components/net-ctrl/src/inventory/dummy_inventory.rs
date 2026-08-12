// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::inventory::SwitchMapping;
use crate::inventory::backend::InventoryBackendApi;
use async_trait::async_trait;
use std::sync::Arc;
use system_error::Error as SysError;

#[derive(Default)]
pub struct InventoryDummyBackend {
    _temp: Arc<SwitchMapping>,
    current: SwitchMapping,
}

impl InventoryDummyBackend {
    pub fn new(data: Arc<SwitchMapping>) -> Self {
        Self {
            _temp: data.clone(),
            current: Self::temp_to_current(data),
        }
    }

    fn temp_to_current(data: Arc<SwitchMapping>) -> SwitchMapping {
        data.as_ref().clone()
    }
}

#[async_trait]
impl InventoryBackendApi for InventoryDummyBackend {
    // todo!("mutex");
    async fn get_switch_mapping(&self) -> Result<SwitchMapping, SysError> {
        Ok(self.current.clone())
    }

    async fn update(&mut self) {
        self.current = InventoryDummyBackend::temp_to_current(self._temp.clone())
    }
}
