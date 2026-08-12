// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

mod backend;
mod dummy_inventory;
mod inventory_ds;
mod inventory_impl;
mod pull_file_backend;
mod push_file_backend;

pub use backend::InventoryBackend;
pub use dummy_inventory::InventoryDummyBackend;
pub use inventory_ds::{SwitchMapping, SwitchModelDetail};
pub use inventory_impl::InventoryConnector;
pub use pull_file_backend::PullInventoryFileBackend;
pub use push_file_backend::PushInventoryFileBackend;
