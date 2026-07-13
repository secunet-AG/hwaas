// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

//! Serial backend that communicates with a process via stdin/stdout

use crate::serial::buffer_sizes::BufferSizes;
use crate::serial::serial_task::{spawn_io_tasks, SerialTasks};
use serde::Deserialize;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{span, Level};

#[derive(Deserialize)]
/// Serial config for serial of type stdio
struct StdioConfig {
    /// Shell command for stdin and stdout
    command: String,

    /// Optional buffer size settings if the defaults are not suiting
    #[serde(flatten)]
    pub buffer_size: BufferSizes,
}

/// Function to convert from `AppConfig` to Stdio specific input and output to
/// put into `SerialTasks`.
pub fn new_with_json(config: serde_json::Value) -> Result<SerialTasks, std::io::Error> {
    let config = serde_json::from_value::<StdioConfig>(config).expect("StdioConfig");

    let mut child = Command::new("sh")
        .arg("-c")
        .arg(&config.command)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    let stdin = child.stdin.take().expect("piped stdin");
    let stdout = child.stdout.take().expect("piped stdout");

    let span = span!(Level::INFO, "stdio@{}", config.command);
    let _enter = span.enter();
    let state = spawn_io_tasks(
        config.command.into(),
        stdout,
        stdin,
        config.buffer_size,
        false,
    );

    Ok(state)
}
