// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::SwitchMapping;
use crate::{InventoryDummyBackend, PullInventoryFileBackend, PushInventoryFileBackend};
use async_trait::async_trait;
use enum_dispatch::enum_dispatch;
use system_error::Error as SysError;

#[async_trait]
#[enum_dispatch]
pub trait InventoryBackendApi {
    async fn get_switch_mapping(&self) -> Result<SwitchMapping, SysError>;

    async fn update(&mut self);
}

#[enum_dispatch(InventoryBackendApi)]
pub enum InventoryBackend {
    InventoryDummyBackend,
    PullInventoryFileBackend,
    PushInventoryFileBackend,
}
