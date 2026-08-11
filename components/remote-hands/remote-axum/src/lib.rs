// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

mod router;
mod server;

pub use router::api_router;
pub use server::{CancelHook, run_axum_server, run_axum_server_with_cleanup};
