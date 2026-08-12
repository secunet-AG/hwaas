// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

pub use api::{get_api, get_router};

mod api;
pub mod app_state;
mod connection_handler;
mod handlers;
mod inventory;
mod network_type_ids;
mod switch;

pub use connection_handler::SwitchMapping;
pub use handlers::setup_data::SetupData;
pub use inventory::*;
