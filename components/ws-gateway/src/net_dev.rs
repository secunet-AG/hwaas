// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use afpacket::tokio::RawPacketStream;
use axum::extract::ws::Message;
use nom::HexDisplay;
use std::fmt::{Debug, Formatter};
use std::io::Result as IResult;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::broadcast;
use tokio::sync::{mpsc, OwnedMutexGuard};
use tracing::{debug, instrument, trace, warn};

pub struct AfNetDev {
    dev_name: String,
    raw_stream: RawPacketStream,
}

impl Debug for AfNetDev {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("AF_PACKET@{}", self.dev_name))
    }
}

// TODO: make this a CLI arg
// this value originates from: 1500 mtu + 14 ethernet header + 4 FCS checksum + 4 VLAN Tag
const MAX_ETHERNET_FRAME_SIZE: usize = 1522;

impl AfNetDev {
    /// Open a network device via AP_PACKET
    ///
    /// # Args
    /// `dev_name` name of the linux network device
    pub fn new(dev_name: String) -> IResult<Self> {
        debug!("Bind to AF_PACKET interface: {}", dev_name);
        let mut raw_stream = RawPacketStream::new()?;
        raw_stream.bind(dev_name.as_str())?;

        Ok(Self {
            dev_name,
            raw_stream,
        })
    }

    /// getter for the network devices raw packet stream
    pub fn get_stream(&self) -> RawPacketStream {
        self.raw_stream.clone()
    }

    #[instrument(skip(self, tx))]
    pub async fn handle_tx(&self, tx: &broadcast::Sender<Message>) {
        loop {
            let mut buf = [0u8; MAX_ETHERNET_FRAME_SIZE];
            let read = match self.get_stream().read(&mut buf).await {
                Ok(s) => s,
                Err(e) => {
                    warn!("Could read from AF_PACKET device: {:?}", e);
                    return;
                }
            };

            if read > MAX_ETHERNET_FRAME_SIZE {
                warn!("packet to large - dropping it");
                continue;
            }

            let msg = Message::Binary(buf[..read].to_vec());

            if let Err(e) = tx.send(msg) {
                warn!("Could not send: {:?}", e);
                return;
            }
            trace!("send over websocket: \n{}", buf[..read].to_hex(12));
        }
    }

    #[instrument(skip(self, rx))]
    pub async fn handle_rx(&self, mut rx: OwnedMutexGuard<mpsc::Receiver<Message>>) {
        while let Some(m) = rx.recv().await {
            let frame = m.into_data();
            let write_res = self.get_stream().write_all(&frame).await;
            if let Err(e) = write_res {
                warn!("Could not write to af_packet device: {:?}", e)
            }
        }
    }
}
