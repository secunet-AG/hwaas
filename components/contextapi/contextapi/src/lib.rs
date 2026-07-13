// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

pub mod api;
mod api_merge_remote;
mod app_config;
mod app_state;
mod context_manager;
mod context_reservation;
mod inventory;
pub(crate) mod net_ctrl;
mod path_params;
pub mod remote_client;
pub mod single_context_api;

#[cfg(test)]
mod tests;

pub use app_config::{ContextApiConfig, WsGatewaySettings};
pub use net_ctrl::NetCtrlClient;

pub const API_VERSION: &str = concat!("v", env!("CARGO_PKG_VERSION_MAJOR"));
