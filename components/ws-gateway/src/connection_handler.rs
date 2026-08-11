// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::fmt::{Debug, Formatter};

use std::sync::Arc;

use dashmap::DashMap;
use dashmap::try_result::TryResult;

use tracing::log::debug;
use tracing::{error, instrument};

use crate::error::ProxyError;
use crate::interface_handler_task::InterfaceHandlerTask;
use crate::interface_streams::InterfaceStreams;

use crate::network_selector::NetworkSelector;

#[derive(Clone)]
pub(crate) struct ConnectionHandler {
    /// prefix of all usable VLAN sub-interfaces
    interface_prefix: String,

    /// Mapping between Networks and corresponding VLAN interfaces
    /// Assumption: InterfaceHandlerTask keeps running all the time and AF_NET_DEV never fails
    net_devs: Arc<DashMap<NetworkSelector, InterfaceHandlerTask>>,
}

impl Default for ConnectionHandler {
    fn default() -> Self {
        Self {
            interface_prefix: "wsn".to_string(),
            net_devs: Default::default(),
        }
    }
}

impl Debug for ConnectionHandler {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("ConnectionHandler[{}]", self.net_devs.len()))
    }
}

impl ConnectionHandler {
    pub(crate) fn new(interface_prefix: String) -> Self {
        ConnectionHandler {
            interface_prefix,
            ..Default::default()
        }
    }

    #[instrument(skip(self))]
    pub(crate) async fn get_or_create(
        &self,
        sel: &NetworkSelector,
    ) -> Result<InterfaceStreams, ProxyError> {
        match self.net_devs.try_get_mut(sel) {
            TryResult::Present(t) => Ok(t.value().attach()),
            TryResult::Locked => Err(ProxyError::TransientError),
            TryResult::Absent => {
                debug!("Going to create IHT");
                let iht = self.create(sel).await?;
                let is = iht.attach();
                if self.net_devs.insert(*sel, iht).is_some() {
                    error!("Replaced an existing IHT");
                }
                Ok(is)
            }
        }
    }

    #[tracing::instrument]
    async fn create(&self, sel: &NetworkSelector) -> Result<InterfaceHandlerTask, ProxyError> {
        let dev_name = format!("{}{}", self.interface_prefix, sel.get_id());

        let iht = InterfaceHandlerTask::new(dev_name.clone()).map_err(|e| {
            error!("Could not create IHT: {}", e);
            ProxyError::InterfaceError
        })?;

        iht.start().await.map_err(|e| {
            error!("Could not start IHT: {:?}", e);
            ProxyError::InterfaceError
        })?;

        Ok(iht)
    }
}
