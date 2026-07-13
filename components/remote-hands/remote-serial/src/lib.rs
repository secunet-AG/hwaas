// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::serial::serial_task::SerialTasks;
use remote_axum::CancelHook;
use std::collections::HashMap;
use std::sync::Arc;

pub mod api;
pub mod app_config;
pub mod app_state;
pub mod serial;

pub fn make_cancel_hook(tasks: HashMap<String, SerialTasks>) -> Option<CancelHook> {
    Some(Arc::new(move || {
        Box::pin({
            let value = tasks.clone();
            async move {
                for (_k, v) in value {
                    v.stop().await;
                }
            }
        })
    }))
}
