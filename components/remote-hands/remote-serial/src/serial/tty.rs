// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

//! Serial backend that talks to Linux TTY layer

use crate::serial::buffer_sizes::BufferSizes;
use crate::serial::serial_task::{SerialTasks, spawn_io_tasks};
use serde::{Deserialize, Serialize};
use serial2_tokio::{CharSize, IntoSettings, KeepSettings, Parity, SerialPort, Settings, StopBits};
use std::io;
use std::path::PathBuf;
use tokio::io::split;
use tracing::{error, instrument};

/// Serial config for a serial device of type **TTY** (e.g. `/dev/ttyUSB0`, `/dev/ttyACM0`).
///
/// # Overview
/// This configuration is used to open and configure a TTY-backed serial port.
/// It combines:
/// - a mandatory device `path`
/// - optional serial port parameters (`TtySettings`)
/// - an optional per-device buffer size override
///
/// # Defaults
/// If a field in `settings` is `None`, the backend keeps its existing/default value.
/// The `Default` implementation for `TtySettings` corresponds to a common 8N1 setup:
/// - 115200 baud
/// - 8 data bits
/// - 1 stop bit
/// - no parity
///
///
/// # Notes
/// - The actual application of settings uses `serial2::Settings`.
/// - `Settings::set_raw()` is applied first to ensure consistent “raw mode” behavior.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TtyConfig {
    /// Filesystem path of the device (e.g. `/dev/ttyUSB0`).
    pub path: String,

    /// Serial settings to apply (baud rate, parity, stop bits, etc.).
    #[serde(flatten)]
    pub settings: TtySettings,

    /// Optional buffer size settings if the defaults are not suiting
    #[serde(flatten)]
    pub buffer_size: BufferSizes,
}

/// Serial port parameters for a TTY device.
///
/// All fields are optional:
/// - When `Some(value)`, the setting is explicitly applied.
/// - When `None`, the corresponding setting is left unchanged (device/default behavior).
///
/// This is useful for:
/// - supporting “TTY-like” devices that do not implement all termios settings,
/// - keeping configurations minimal when defaults are sufficient.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TtySettings {
    /// Baud rate (e.g. 9600, 115200).
    pub baud_rate: Option<u32>,

    /// Character size / data bits (typically 7 or 8).
    pub char_size: Option<CharSize>,

    /// Stop bits (usually 1).
    pub stop_bits: Option<StopBits>,

    /// Parity bit configuration.
    pub parity: Option<Parity>,
}

impl Default for TtySettings {
    /// Default settings for a typical serial console (8N1 at 115200 baud).
    fn default() -> Self {
        Self {
            baud_rate: Some(115_200),
            char_size: Some(CharSize::Bits8),
            stop_bits: Some(StopBits::One),
            parity: Some(Parity::None),
        }
    }
}

/// Apply this configuration to `serial2::Settings`.
///
/// # Behavior
/// - Always calls `settings.set_raw()` first (recommended by `serial2`) to establish
///   consistent non-canonical/raw behavior.
/// - Applies only those parameters that are `Some(...)`.
///
/// # Error handling
/// Returns `io::Error` if applying a setting fails (e.g. the device does not support
/// a requested termios setting).
///
/// In test environments using PTYs, applying baud rate / termios settings may fail.
/// In such cases, consider using `serial2::KeepSettings` for the test devices.
impl IntoSettings for TtyConfig {
    fn apply_to_settings(self, settings: &mut Settings) -> io::Result<()> {
        // Recommended: set raw mode first, then configure individual settings.
        settings.set_raw();

        if let Some(b) = self.settings.baud_rate {
            settings.set_baud_rate(b)?;
        }
        if let Some(c) = self.settings.char_size {
            settings.set_char_size(c);
        }
        if let Some(s) = self.settings.stop_bits {
            settings.set_stop_bits(s);
        }
        if let Some(p) = self.settings.parity {
            settings.set_parity(p);
        }

        Ok(())
    }
}

/// Function to convert from path only to TTY specific input and output to put
/// into `SerialTasks`. Used for dynamic USB OTG serial connections.
pub fn new_with_path(path: String) -> Result<SerialTasks, io::Error> {
    let config = TtyConfig {
        path,
        settings: Default::default(),
        buffer_size: Default::default(),
    };
    new_with_json(serde_json::to_value(config)?)
}

/// Function to convert from `AppConfig` to TTY specific input and output to put
/// into `SerialTasks`. Used for static serial connections.
pub fn new_with_json(config: serde_json::Value) -> Result<SerialTasks, io::Error> {
    let config = serde_json::from_value::<TtyConfig>(config)
        .inspect_err(|error| error!(?error, "Wrong tty config"))?;

    let serial: SerialPort = open_serial_with_fallback(config.clone())
        .inspect_err(|_| error!("Could not open serial port"))?;

    let (reader, writer) = split(serial);

    Ok(spawn_io_tasks(
        config.path.into(),
        reader,
        writer,
        config.buffer_size,
        false,
    ))
}

/// Open a serial TTY device with a robust fallback strategy.
///
/// # Behavior
/// This function first tries to open and configure the device using the full
/// `TtyConfig` (via `open_serial_tty`). This applies the requested termios settings
/// such as baud rate, character size, stop bits, and parity.
///
/// If that fails (commonly on PTYs or TTY-like devices that do not support some
/// termios options), it logs a warning and retries opening the device using
/// `KeepSettings`, which leaves the existing device settings untouched.
///
/// # When the fallback is used
/// The fallback path is typically triggered when:
/// - The device is a PTY or pseudo-terminal used in tests.
/// - The underlying driver does not support changing baud rate or other settings.
/// - The OS rejects one or more termios options with
///   "failed to apply some or all settings".
///
/// # Logging
/// On fallback, a warning is emitted including:
/// - the device path
/// - the requested baud rate (if any)
/// - the original error returned by the failed attempt
///
/// # Returns
/// - `Ok(SerialPort)` if the device could be opened either with full settings
///   or via the fallback using `KeepSettings`.
/// - `Err(io::Error)` only if both attempts fail.
///
/// # Notes
/// - This makes production code work with real serial devices while still allowing
///   tests to run against PTYs or other TTY-like devices.
/// - The returned `SerialPort` implements `AsyncRead` and `AsyncWrite` and can be
///   directly integrated into the Tokio IO pipeline.
#[instrument(level = "info", skip(tty_config), fields(tty = %tty_config.path))]
pub fn open_serial_with_fallback(tty_config: TtyConfig) -> io::Result<SerialPort> {
    match open_serial_tty(tty_config.clone()) {
        Ok(p) => Ok(p),
        Err(_e) => {
            // Common when device doesn't support some termios settings (e.g. PTY).
            tracing::warn!("failed to apply settings, retrying with KeepSettings");
            SerialPort::open(tty_config.path, KeepSettings)
        }
    }
}

/// Open and configure a serial TTY device (e.g. `/dev/ttyUSB0`) using `serial2_tokio`.
///
/// This function is Linux/Unix-oriented and assumes the device is a real serial port.
/// For PTYs or terminal devices, configuration semantics may differ.
#[instrument(level = "info", skip(tty_config), fields(tty = %tty_config.path))]
pub fn open_serial_tty(tty_config: TtyConfig) -> io::Result<SerialPort> {
    if !PathBuf::from(&tty_config.path).exists() {
        error!(?tty_config.path, "tty path does not exist");
        return Err(io::ErrorKind::NotFound.into());
    }

    // Open the device
    let port = SerialPort::open(tty_config.path.clone(), tty_config)
        .inspect_err(|error| error!(?error, "Failed to open serial port with settings"))?;
    Ok(port)
}
