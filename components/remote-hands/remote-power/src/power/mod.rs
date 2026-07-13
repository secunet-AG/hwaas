// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

pub mod custom;

use crate::app_config::PowerControlConfig;
use axum::async_trait;
use custom::CustomPowerControl;
use enum_dispatch::enum_dispatch;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// The `PowerState` a power interface can have. Usually `On` and `Off` are enough.
/// Some interfaces do have a state of `Reset`, which will change to `On` after
/// a while. `Unknown` is used for the default implementation of `query()`.
pub enum PowerState {
    On,
    Off,
    Reset,
    Unknown,
}

#[async_trait]
#[enum_dispatch]
/// Trait to enforce which functions each power type needs to implement.
pub trait PowerControl: Send + Sync {
    /// Encapsulates the logic to power on an interface.
    async fn power_on(&mut self) -> Result<(), std::io::Error>;
    /// Encapsulates the logic to power off an interface.
    async fn power_off(&mut self) -> Result<(), std::io::Error>;

    /// Encapsulates the logic to reset an interface.
    /// The default implementation is `power_off` + `power_on`.
    async fn reset(&mut self) -> Result<(), std::io::Error> {
        self.power_off().await?;
        self.power_on().await
    }

    /// Encapsulates the logic to query a machine for its `PowerState`.
    /// If not implemented, `PowerState::Unknown` will be returned.
    async fn query(&mut self) -> Result<PowerState, std::io::Error> {
        Ok(PowerState::Unknown)
    }
}

#[enum_dispatch(PowerControl)]
/// An enum to list all power control backends we support.
/// Currently, we only support the "custom" backend.
pub enum PowerControlBackend {
    CustomPowerControl,
}

/// This function is used (in main.rs) to parse the correct `PowerControlBackend`
/// depending on the power type.
/// If the power type is unknown, we immediately terminate with an error message.
pub fn new(control_config: PowerControlConfig) -> PowerControlBackend {
    match &control_config.power_type[..] {
        "custom" => {
            let config =
                serde_json::from_value(control_config.config).expect("custom control_config");
            PowerControlBackend::from(CustomPowerControl::new(config))
        }
        _ => {
            panic!("Unknown power type: {}", control_config.power_type);
        }
    }
}
