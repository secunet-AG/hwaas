// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::CliArgs;
use crate::error::ClientError;
use crate::tap::TapDev;
use futures_util::StreamExt;
use sd_notify::NotifyState;
use tokio_tungstenite::connect_async;
use url::Url;

/// Client data
pub struct WsL2Client {
    args: CliArgs,
    dev: TapDev,
}

/// Proxy in websocket client role
impl WsL2Client {
    /// Create a new client
    pub(crate) fn new(args: CliArgs) -> Result<Self, ClientError> {
        // Open a net device in "raw" mode (AF_PACKET)
        let dev = TapDev::new(args.dev.clone(), args.mtu)?;
        Ok(Self { args, dev })
    }

    /// start proxying l2 packets over websocket
    /// after establishing the websocket connection
    pub async fn start(self) -> Result<(), ClientError> {
        // prepare the websocket server URL
        let case_url = Url::parse(&self.args.address.to_string())?;

        // establish the websocket connection
        let (ws_stream, _) = connect_async(case_url.to_string()).await?;

        info!("WebSocket connected");

        // If spawning the app via systemd report when everything is set up
        let _ = sd_notify::notify(true, &[NotifyState::Ready])
            .map_err(|e| warn!("Could not use sd_notify: {:?}", e));

        // split websocket into read and write parts to handel them concurrently
        let (ws_tx, ws_rx) = ws_stream.split();

        self.dev.handle_streams(ws_tx, ws_rx).await;
        debug!("websocket stream terminated");
        Ok(())
    }
}
