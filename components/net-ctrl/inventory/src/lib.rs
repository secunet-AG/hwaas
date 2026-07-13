// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

mod backend;
mod dummy_inventory;
mod inventory;
mod inventory_ds;
mod pull_file_backend;
mod push_file_backend;

pub use crate::inventory::InventoryConnector;
pub use backend::InventoryBackend;
pub use backend::InventoryBackendApi;
pub use dummy_inventory::InventoryDummyBackend;
pub use inventory_ds::{SwitchMapping, SwitchModelDetail};
pub use pull_file_backend::PullInventoryFileBackend;
pub use push_file_backend::PushInventoryFileBackend;
