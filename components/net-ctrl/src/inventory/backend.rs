// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::inventory::SwitchMapping;
use crate::inventory::{InventoryDummyBackend, PullInventoryFileBackend, PushInventoryFileBackend};
use async_trait::async_trait;
use enum_dispatch::enum_dispatch;
use system_error::Error as SysError;

#[async_trait]
#[enum_dispatch]
pub trait InventoryBackendApi {
    async fn get_switch_mapping(&self) -> Result<SwitchMapping, SysError>;
}

#[enum_dispatch(InventoryBackendApi)]
pub enum InventoryBackend {
    InventoryDummyBackend,
    PullInventoryFileBackend,
    PushInventoryFileBackend,
}
