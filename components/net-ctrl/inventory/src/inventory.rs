// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::backend::InventoryBackendApi;
use crate::{InventoryBackend, SwitchMapping, SwitchModelDetail};
use libc::ENXIO;
use network_type_ids::SwitchID;
use system_error::Error as SysError;
use tracing::warn;

/// The inventory connector utilizes an Adapter design pattern.
/// Regardless how the information are stored and how to retrieve them, the
/// InventoryConnector poses a common way for obtaining the desired Information.
pub struct InventoryConnector {
    // Contains Backend that holds a map of SwitchIDs to SwitchSelectors
    backend: InventoryBackend,
}

impl InventoryConnector {
    /// This constructor consumes an arbitrary [`InventoryBackend`].
    /// The Backend defines the strategy how to retrieve desired information.
    pub fn new(backend: InventoryBackend) -> Self {
        Self { backend }
    }

    /// This function could be used to get a [`SwitchModelDetail`] corresponding to a given
    /// [`SwitchID`]. This function is async as it is involved in data flow.
    pub async fn get_switch_model_details(
        &self,
        switch_id: &SwitchID,
    ) -> Result<SwitchModelDetail, SysError> {
        // Retrieve SwitchSelector for a switch from Map.
        self.get_switch_mapping()
            .await?
            .get(switch_id)
            .cloned()
            .ok_or_else(|| {
                warn!("no inventory item for '{}'", switch_id.to_string());
                SysError::from_raw_os_error(ENXIO)
            })
    }

    /// This function could be used to get the full [`SwitchMapping`].
    /// This function is async as it is involved in data flow.
    pub async fn get_switch_mapping(&self) -> Result<SwitchMapping, SysError> {
        // Retrieve mapping of SwitchID's to SwitchSelector's from InventoryBackend.
        self.backend.get_switch_mapping().await
    }
}
