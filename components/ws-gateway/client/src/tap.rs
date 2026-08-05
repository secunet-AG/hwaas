// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use etherparse::PacketHeaders;
use futures_util::{Sink, SinkExt, StreamExt};
use nom::HexDisplay;
use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::select;
use tokio_tun::{Tun, TunBuilder, result::Result};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::Result as TResult;

pub struct TapDev {
    _dev_name: String,
    tap: Tun,
    _mtu: u32,
}

const MTU_SIZE: usize = 1500;

impl TapDev {
    pub fn new(dev_name: String, mtu: u32) -> Result<Self> {
        match dev_name.len() {
            0 => debug!("Creating new TAP interface"),
            _ => debug!("Creating TAP interface {}", dev_name),
        }

        // Construct a tap interface and set it up
        let tap = TunBuilder::new()
            .name(dev_name.as_str())
            .tap(true)
            .packet_info(false)
            .mtu(mtu as i32)
            .up()
            .try_build()?;

        // Get the actual name the kernel gave to us.
        // The builder name is more a suggestion.
        let _dev_name = tap.name().to_string();
        info!("Created TAP interface: {}", &dev_name);

        Ok(Self {
            _dev_name,
            tap,
            _mtu: mtu,
        })
    }

    pub async fn handle_streams<T: Sink<Message>, R: StreamExt<Item = TResult<Message>>>(
        self,
        ws_tx: T,
        ws_rx: R,
    ) where
        <T as futures_util::Sink<Message>>::Error: std::fmt::Debug,
        T: std::marker::Unpin,
        R: std::marker::Unpin,
    {
        let (tap_rx, tap_tx) = tokio::io::split(self.tap);

        select! {
            _ = Self::handle_tx(ws_tx, tap_rx) => debug!("rx terminated"),
            _ = Self::handle_rx(ws_rx, tap_tx) => debug!("tx terminated")
        };
    }

    #[instrument(skip(tx, tap_rx))]
    async fn handle_tx<S: Sink<Message>>(mut tx: S, mut tap_rx: ReadHalf<Tun>)
    where
        <S as futures_util::Sink<Message>>::Error: std::fmt::Debug,
        S: std::marker::Unpin,
    {
        loop {
            let mut buf = [0u8; MTU_SIZE];
            let read = match tap_rx.read(&mut buf).await {
                Ok(s) => s,
                Err(e) => {
                    warn!("Could read from AF_PACKET device: {:?}", e);
                    return;
                }
            };

            if read > MTU_SIZE {
                warn!("packet to large - dropping it");
                continue;
            }

            let msg = Message::Binary(buf[..read].to_vec().into());

            if let Err(e) = tx.send(msg).await {
                warn!("Could not send: {:?}", e);
                return;
            }
            trace!("send over websocket: \n{}", buf[..read].to_hex(12));
        }
    }

    #[instrument(skip(rx, tap_tx))]
    async fn handle_rx<S: StreamExt<Item = TResult<Message>>>(mut rx: S, mut tap_tx: WriteHalf<Tun>)
    where
        S: std::marker::Unpin,
    {
        while let Some(m) = rx.next().await {
            match m {
                Ok(msg) => {
                    let frame = &*msg.into_data();
                    let eth_msg = PacketHeaders::from_ethernet_slice(frame);

                    match eth_msg {
                        Ok(s) => trace!("Parsed: {:?}", s.link),
                        Err(e) => debug!("Parse failed: {:?}", e),
                    };

                    let write_res = tap_tx.write_all(frame).await;
                    if let Err(e) = write_res {
                        warn!("Could not write to af_packet device: {:?}", e)
                    }
                }
                Err(e) => {
                    warn!("could not get message from ws: {:?}", e);
                    break;
                }
            };
        }
    }
}
