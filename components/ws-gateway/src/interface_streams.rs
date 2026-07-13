// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use axum::extract::ws::Message;
use std::fmt::{Debug, Formatter};

use crate::interface_handler_task::InterfaceHandlerTask;
use tokio::sync::{broadcast, mpsc};

pub struct InterfaceStreams {
    pub(crate) tx: mpsc::Sender<Message>,
    pub(crate) rx: broadcast::Receiver<Message>,
    pub(crate) task_name: String,
}

impl InterfaceStreams {
    pub(crate) fn get(self) -> (broadcast::Receiver<Message>, mpsc::Sender<Message>) {
        (self.rx, self.tx)
    }

    pub(crate) fn get_iht_name(&self) -> String {
        self.task_name.clone()
    }
}

impl Debug for InterfaceStreams {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Streams[{}]", self.task_name))
    }
}

impl From<&InterfaceHandlerTask> for InterfaceStreams {
    fn from(iht: &InterfaceHandlerTask) -> Self {
        iht.attach()
    }
}
