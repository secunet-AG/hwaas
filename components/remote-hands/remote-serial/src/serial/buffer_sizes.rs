// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct BufferSizes {
    /// Optional ring buffer size override (in bytes) for this device.
    /// This size determines the length of the preserved and served serial history.
    ///
    /// If set, this value is used for the per-device ring buffer capacity.
    /// If omitted, the service-wide/default ring buffer sizing is used.
    pub ring_buffer_size: usize,

    /// Optional broadcast channel size override (in bytes) for this device.
    ///
    /// If set, this value is used for the per-device live broadcast channel capacity.
    /// If omitted, the service-wide/default broadcast capacity is used.
    pub broadcast_buffer_size: usize,
}

impl Default for BufferSizes {
    fn default() -> Self {
        Self {
            ring_buffer_size: 64 * 1024,
            broadcast_buffer_size: 4096,
        }
    }
}
