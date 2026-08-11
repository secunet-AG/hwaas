// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::serial::byte_ring::ByteRing;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast, mpsc};

/// Shared state for one serial device.
///
/// - The ring buffer is used for HTTP GET (snapshot).
/// - The broadcast channel is used for WebSocket live-streaming.
/// - Serial writes are funneled through `write_tx` to a single writer task.
#[derive(Clone)]
pub struct SerialState {
    pub ring: Arc<RwLock<ByteRing>>,
    pub write_tx: mpsc::Sender<Vec<u8>>,

    /// Broadcasts chunks of bytes read from serial (and optionally echoed writes).
    ///
    /// Each WebSocket connection calls `subscribe()` to receive new chunks
    /// from the moment it connects (no history).
    pub ws_tx: broadcast::Sender<Arc<[u8]>>,
}
