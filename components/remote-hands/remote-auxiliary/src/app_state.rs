// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, sync::Arc};
use tokio::process::Command;
use tokio::sync::Mutex;
use tracing::error;

use crate::api::activation_info::AuxiliaryDevice;
use crate::auxiliary::{AuxConfig, AuxState};

#[derive(Clone)]
/// AppState: Contains a HashMap of `DeviceState`s.
pub struct AppState {
    pub aux_devices: Arc<HashMap<String, DeviceState>>,
}

#[derive(Clone)]
/// DeviceState: Contains the actual `AuxConfig` and the current activation state.
/// The `aux_state` is encapsulated in a Mutex, so that we do not have a raise
/// when changing it.
pub struct DeviceState {
    pub aux_config: AuxConfig,
    pub aux_state: Arc<Mutex<AuxState>>,
}

impl Default for AppState {
    /// Initilization with an empty Hashmap.
    /// Is needed for OpenAPI spec generation, see `openapi-generator.rs`.
    fn default() -> Self {
        AppState {
            aux_devices: Arc::new(HashMap::new()),
        }
    }
}

impl AppState {
    /// Initialization with given auxiliary config.
    /// Encapsulate config in `DeviceState` with the state `AuxState::Off`, since
    /// all auxiliary devices should by convention be off in the beginning.
    /// This is not enforced by `remote-hands`, but by the ContextAPIs machine
    /// initialization and context teardown mechanisms.
    pub fn new(aux_devices: HashMap<String, AuxConfig>) -> Self {
        let mut device_state: HashMap<String, DeviceState> = HashMap::new();
        for (aux, aux_config) in aux_devices {
            device_state.insert(
                aux,
                DeviceState {
                    aux_config,
                    aux_state: Arc::new(Mutex::new(AuxState::Off)),
                },
            );
        }
        AppState {
            aux_devices: Arc::new(device_state),
        }
    }

    /// Return human-readable auxiliary device information for all configured
    /// devices. This contains all device IDs/names and their activation state.
    pub async fn get_aux_infos(&self) -> Option<Vec<AuxiliaryDevice>> {
        let aux_devices = HashMap::clone(&self.aux_devices).into_iter();
        let mut v = Vec::new();
        for (aux, inner) in aux_devices {
            let state = inner
                .query()
                .await
                .map_err(|e| error!("{}", format!("query state for '{aux}' failed: {e}")))
                .ok()?;
            let act = match state {
                AuxState::Off => AuxiliaryDevice::off(&inner.aux_config.id),
                AuxState::On => AuxiliaryDevice::on(&inner.aux_config.id),
            };
            v.push(act);
        }
        Some(v)
    }
}

impl DeviceState {
    /// Activate an auxiliary device.
    /// Executed the configured activation command with true.
    /// Sets the internal state to `AuxState::On`.
    pub async fn power_on(&self) -> Result<(), std::io::Error> {
        Self::run_command(&self.aux_config.cmd, "true").await?;
        let mut state = self.aux_state.lock().await;
        *state = AuxState::On;
        Ok(())
    }

    /// Deactivate an auxiliary device.
    /// Executed the configured activation command with false.
    /// Sets the internal state to `AuxState::Off`.
    pub async fn power_off(&self) -> Result<(), std::io::Error> {
        Self::run_command(&self.aux_config.cmd, "false").await?;
        let mut state = self.aux_state.lock().await;
        *state = AuxState::Off;
        Ok(())
    }

    /// Simply queries the internal activation state on one auxiliary device.
    pub async fn query(&self) -> Result<AuxState, std::io::Error> {
        let state = self.aux_state.lock().await;
        Ok(*state)
    }

    /// Helper function to run the activation command on shell.
    /// Can throw an error if the command failed.
    async fn run_command(command: &str, b: &str) -> Result<(), std::io::Error> {
        let status = Command::new("sh")
            .arg("-c")
            .arg(command)
            .arg("path") // placeholder since $1 must be second arg after command
            .arg(b)
            .kill_on_drop(true)
            .status()
            .await?;
        if status.success() {
            Ok(())
        } else {
            error!(status = %status, command = %command, "custom command failed");
            Err(std::io::ErrorKind::Other.into())
        }
    }
}
