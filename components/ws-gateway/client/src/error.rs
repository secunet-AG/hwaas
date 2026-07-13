// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error(
        "Insufficient Permissions to open a TAP device (capability SYS_NET_ADMIN required) {0}"
    )]
    TapCreation(#[from] tokio_tun::result::Error),
    #[error("Bad URL format")]
    BadURL(#[from] url::ParseError),
    #[error("Establishing websocket connection failed")]
    ConnectionFailure(#[from] tokio_tungstenite::tungstenite::Error),
}
