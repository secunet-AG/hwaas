// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::power::PowerControl;
use axum::async_trait;
use serde::Deserialize;
use tokio::process::Command;
use tracing::{error, instrument, warn};

use super::PowerState;

/// Custom power config that provides 4 commands: the mandatory `on` and `off`
/// commands and the optional `reset` and `query` commands.
/// This type is needed when configuring the `remote-power` service for a
/// machine, as input data.
#[derive(Deserialize)]
pub struct CustomPowerConfig {
    pub on: String,
    pub off: String,
    pub reset: Option<String>,
    pub query: Option<String>,
}

/// Internal meta information about the state of the power interface.
/// Contains the config with all commands and a cached `PowerState` as backup,
/// when the `query` command is not implemented.
pub struct CustomPowerControl {
    config: CustomPowerConfig,
    /// Fallback for when no `query` command is available.
    /// This value starts out as [`PowerState::Unknown`],
    /// and gets updated by [`Self::power_on`] and
    /// [`Self::power_off`]. On failure we always set
    /// this value back to [`PowerState::Unknown`].
    cached_state: PowerState,
}

impl CustomPowerControl {
    /// Initialization. The cached state simply starts as `PowerState::Unknown`,
    /// since the value is only used when no `query` command is provided anyway,
    /// so that we cannot query the actual state.
    pub fn new(config: CustomPowerConfig) -> Self {
        CustomPowerControl {
            config,
            cached_state: PowerState::Unknown,
        }
    }
}

#[async_trait]
impl PowerControl for CustomPowerControl {
    /// Execute `on` command from config.
    /// Change cached state accordingly.
    async fn power_on(&mut self) -> Result<(), std::io::Error> {
        if let Err(e) = run_command(&self.config.on).await {
            self.cached_state = PowerState::Unknown;
            Err(e)
        } else {
            self.cached_state = PowerState::On;
            Ok(())
        }
    }

    /// Execute `off` command from config.
    /// Change cached state accordingly.
    async fn power_off(&mut self) -> Result<(), std::io::Error> {
        if let Err(e) = run_command(&self.config.off).await {
            self.cached_state = PowerState::Unknown;
            Err(e)
        } else {
            self.cached_state = PowerState::Off;
            Ok(())
        }
    }

    /// Execute `reset` command from config.
    /// Change cached state accordingly.
    /// Execute `power_off` + `power_on` as fallback if not configured.
    async fn reset(&mut self) -> Result<(), std::io::Error> {
        if let Some(reset) = &self.config.reset {
            if let Err(e) = run_command(reset).await {
                self.cached_state = PowerState::Unknown;
                Err(e)
            } else {
                // Assuming a success means that the power must be on
                self.cached_state = PowerState::On;
                Ok(())
            }
        } else {
            self.power_off().await?;
            self.power_on().await
        }
    }

    #[instrument(skip(self))]
    /// Execute `query` command from config.
    /// Since the returned stdout must be parsed into `PowerState`,
    /// the called script must return "on", "off", "reset", or "unknown".
    /// Be aware that only the string "unknown" leads to `PowerState::Unknown`,
    /// every invalid output will lead to an error instead.
    /// Return cached state when `query` is not configured.
    async fn query(&mut self) -> Result<PowerState, std::io::Error> {
        let Some(query) = &self.config.query else {
            warn!("no configured query command: falling back to cached power state");
            return Ok(self.cached_state);
        };
        let output = Command::new("sh")
            .arg("-c")
            .arg(query)
            .kill_on_drop(true)
            .output()
            .await?;
        if output.status.success() {
            let stdout = output.stdout;
            if stdout.starts_with(b"on") {
                Ok(PowerState::On)
            } else if stdout.starts_with(b"off") {
                Ok(PowerState::Off)
            } else if stdout.starts_with(b"reset") {
                Ok(PowerState::Reset)
            } else if stdout.starts_with(b"unknown") {
                Ok(PowerState::Unknown)
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                error!(stderr = ?stderr, "invalid output for custom query command");
                Err(std::io::ErrorKind::InvalidData.into())
            }
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!(
            status = %output.status,
            command = %query,
            stderr = ?stderr,
            "custom query command failed"
            );
            Err(std::io::ErrorKind::Other.into())
        }
    }
}

/// Helper function to execute a command on the shell.
/// If the status is okay, we simply return without looking at the output.
/// If not, we log the stderr and return an error.
async fn run_command(command: &str) -> Result<(), std::io::Error> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .kill_on_drop(true)
        .output()
        .await?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!(
            status = %output.status,
            command = %command,
            stderr = ?stderr,
            "custom command failed"
        );
        Err(std::io::ErrorKind::Other.into())
    }
}
