// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

//! Serial backend that communicates with a process via stdin/stdout

use super::{SerialInput, SerialOutput};
use axum::async_trait;
use serde::Deserialize;
use std::process::Stdio;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::{ChildStdin, ChildStdout, Command},
};

#[derive(Deserialize)]
/// Serial config for serial of type stdio
struct StdioConfig {
    /// Shell command for stdin and stdout
    command: String,
}

pub struct StdioInput {
    stdout: ChildStdout,
}

pub struct StdioOutput {
    stdin: ChildStdin,
}

/// Function to convert from `AppConfig` to Stdio specific input and output to
/// put into `SerialState`.
pub fn new_with_json(
    config: serde_json::Value,
) -> Result<(StdioInput, StdioOutput), std::io::Error> {
    let config = serde_json::from_value::<StdioConfig>(config).expect("StdioConfig");
    let child = Command::new("sh")
        .arg("-c")
        .arg(&config.command)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    Ok((
        StdioInput {
            stdout: child.stdout.expect("child.stdout"),
        },
        StdioOutput {
            stdin: child.stdin.expect("child.stdin"),
        },
    ))
}

#[async_trait]
impl SerialInput for StdioInput {
    async fn read(&mut self) -> Result<Vec<u8>, std::io::Error> {
        let mut buf = [0; 256];
        match self.stdout.read(&mut buf).await? {
            0 => Err(std::io::ErrorKind::UnexpectedEof.into()),
            bytes_read => Ok(Vec::from(&buf[..bytes_read])),
        }
    }
}

#[async_trait]
impl SerialOutput for StdioOutput {
    async fn write(&mut self, data: &[u8]) -> Result<(), std::io::Error> {
        let mut offset = 0;
        while offset < data.len() {
            match self.stdin.write(&data[offset..]).await? {
                0 => return Err(std::io::ErrorKind::UnexpectedEof.into()),
                bytes_written => offset += bytes_written,
            }
        }
        Ok(())
    }
}
