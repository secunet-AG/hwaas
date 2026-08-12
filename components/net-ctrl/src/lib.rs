// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

pub use api::{get_api, get_router};

mod api;
pub mod app_state;
pub mod connection_handler;
mod handlers;
pub mod inventory;
pub mod network_type_ids;
pub mod switch;

pub use connection_handler::SwitchMapping;
pub use handlers::setup_data::SetupData;
pub use inventory::*;
