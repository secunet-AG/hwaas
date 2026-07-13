// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use axum::extract::ws::Message;
use std::fmt::{Debug, Display, Formatter};
use std::io;
use std::sync::Arc;
use tokio::select;
use tracing::{debug, error, warn};

use crate::net_dev::AfNetDev;
use tokio::sync::{broadcast, mpsc, Mutex};
use tokio::task::{Builder, JoinHandle};

use crate::interface_streams::InterfaceStreams;

/// Errors returned if starting the main async task within an
/// InterfaceHandlerTask fails.
#[derive(Debug)]
pub(crate) enum InterfaceHandlerTaskError {
    /// starting the main taks fail - origin: async runtime
    IoError,
    /// the interface is already connected to the streams
    /// because a task was alreay started.
    Busy,
}

/// The [`InterfaceHandlerTask`] encapsulates the real network interface stream.
/// It expose clonable streams used to connect concurrently to a real interface.
#[derive(Clone)]
pub(crate) struct InterfaceHandlerTask {
    inner_tx: broadcast::Sender<Message>,
    inner_rx: Arc<Mutex<mpsc::Receiver<Message>>>,
    outer_tx: mpsc::Sender<Message>,
    outer_rx: Arc<broadcast::Receiver<Message>>,
    dev: Arc<Mutex<AfNetDev>>,
    interface_name: String,
}

impl Display for InterfaceHandlerTask {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("IHT[{}]", self.interface_name))
    }
}

impl Debug for InterfaceHandlerTask {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl InterfaceHandlerTask {
    /// Construct a new [`InterfaceHandlerTask`] for a real linux network interface,
    /// which was specified via the parameter `interface_name`.
    pub fn new(interface_name: String) -> Result<Self, io::Error> {
        let (outer_tx, inner_rx) = mpsc::channel(20);
        let (inner_tx, outer_rx) = broadcast::channel(20);
        let inner_rx = Arc::new(Mutex::new(inner_rx));
        let outer_rx = Arc::new(outer_rx);

        match AfNetDev::new(interface_name.clone()).map(|i| Arc::new(Mutex::new(i))) {
            Ok(dev) => Ok(Self {
                inner_rx,
                inner_tx,
                outer_tx,
                outer_rx,
                dev,
                interface_name,
            }),
            Err(e) => {
                error!("Could not open device: {}", e);
                Err(e)
            }
        }
    }

    #[tracing::instrument]
    pub async fn start(
        &self,
    ) -> Result<JoinHandle<Result<(), InterfaceHandlerTaskError>>, InterfaceHandlerTaskError> {
        let s = self.clone();

        Builder::new()
            .name(self.to_string().as_str())
            .spawn(async move { s.start_inner().await })
            .map_err(|e| {
                warn!("Could not start Task to serve interface streams: {}", e);
                InterfaceHandlerTaskError::IoError
            })
    }

    async fn start_inner(&self) -> Result<(), InterfaceHandlerTaskError> {
        let dev = self.dev.clone().try_lock_owned().map_err(|e| {
            warn!(
                "Could not get lock for NETDEV {}: {}",
                self.interface_name, e
            );
            InterfaceHandlerTaskError::Busy
        })?;

        select! {
            _ = dev.handle_tx(&self.inner_tx) => debug!("rx terminated"),
            _ = dev.handle_rx(self.inner_rx.clone().lock_owned().await) => debug!("tx terminated")
        }

        Ok(())
    }

    pub fn attach(&self) -> InterfaceStreams {
        InterfaceStreams {
            rx: self.outer_rx.resubscribe(),
            tx: self.outer_tx.clone(),
            task_name: self.to_string(),
        }
    }
}
