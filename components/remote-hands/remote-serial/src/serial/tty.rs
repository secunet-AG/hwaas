// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

//! Serial backend that talks to Linux' TTY layer

use std::sync::Arc;

use super::{SerialInput, SerialOutput};
use axum::async_trait;
use serde::Deserialize;
use serial2_tokio::{CharSize, Parity, SerialPort, Settings, StopBits};

#[derive(Deserialize)]
/// Serial config for serial of type tty
struct TtyConfig {
    path: String,
    baud_rate: Option<u32>,
    char_size: Option<u8>,
    stop_bits: Option<u8>,
    parity: Option<Parity>,
}

pub struct TtyInput {
    tty: Arc<SerialPort>,
}

pub struct TtyOutput {
    tty: Arc<SerialPort>,
}

/// Function to convert from path only to TTY specific input and output to put
/// into `SerialState`. Used for dynamic USB OTG serial connections.
pub fn new_with_path(path: String) -> Result<(TtyInput, TtyOutput), std::io::Error> {
    let config = TtyConfig {
        path,
        baud_rate: None,
        char_size: None,
        stop_bits: None,
        parity: None,
    };
    new(config)
}

/// Function to convert from `AppConfig` to TTY specific input and output to put
/// into `SerialState`. Used for static serial connections.
pub fn new_with_json(config: serde_json::Value) -> Result<(TtyInput, TtyOutput), std::io::Error> {
    let config = serde_json::from_value::<TtyConfig>(config).expect("TtyConfig");
    new(config)
}

/// Helper function to convert TtyConfig to input and output for `SerialState`.
fn new(config: TtyConfig) -> Result<(TtyInput, TtyOutput), std::io::Error> {
    let tty = SerialPort::open(config.path.clone(), |mut settings: Settings| {
        settings.set_raw();
        if let Some(baud_rate) = config.baud_rate {
            if let Err(e) = settings.set_baud_rate(baud_rate) {
                panic!("Unsupported baud rate {baud_rate}: {e}");
            }
        }
        if let Some(char_size) = config.char_size {
            let char_size = match char_size {
                5 => CharSize::Bits5,
                6 => CharSize::Bits6,
                7 => CharSize::Bits7,
                8 => CharSize::Bits8,
                _ => panic!("Unsupported char_size configuration: {char_size}"),
            };
            settings.set_char_size(char_size);
        }
        if let Some(stop_bits) = config.stop_bits {
            let stop_bits = match stop_bits {
                1 => StopBits::One,
                2 => StopBits::Two,
                _ => panic!("Unsupported stop_bits configuration: {stop_bits}"),
            };
            settings.set_stop_bits(stop_bits);
        }
        if let Some(parity) = config.parity {
            settings.set_parity(parity);
        }
        Ok(settings)
    })
    .inspect_err(|_e| {
        if let Ok(available_ports) = SerialPort::available_ports() {
            let available_ports = available_ports
                .iter()
                .filter_map(|buf| buf.as_path().to_str());
            tracing::error!(
                "cannot open {}, available serial devices: {:?}",
                config.path,
                available_ports,
            );
        }
    })?;

    let tty = Arc::new(tty);
    Ok((TtyInput { tty: tty.clone() }, TtyOutput { tty }))
}

#[async_trait]
impl SerialInput for TtyInput {
    async fn read(&mut self) -> Result<Vec<u8>, std::io::Error> {
        let mut buf = [0; 256];
        match self.tty.read(&mut buf).await? {
            0 => Err(std::io::ErrorKind::UnexpectedEof.into()),
            bytes_read => Ok(Vec::from(&buf[..bytes_read])),
        }
    }
}

#[async_trait]
impl SerialOutput for TtyOutput {
    async fn write(&mut self, data: &[u8]) -> Result<(), std::io::Error> {
        self.tty.write_all(data).await
    }
}
