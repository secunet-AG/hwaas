// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::api::SerialID;
use crate::serial::buffer_sizes::BufferSizes;
use crate::serial::byte_ring::ByteRing;
use crate::serial::serial_state::SerialState;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::{Mutex, RwLock, broadcast, mpsc};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{info, instrument, warn};

/// A running serial task pair (reader + writer) that can be stopped.
#[derive(Clone)]
pub struct SerialTasks {
    id: String,
    shutdown: CancellationToken,
    reader_jh: Arc<Mutex<JoinHandle<()>>>,
    writer_jh: Arc<Mutex<JoinHandle<()>>>,
    pub state: SerialState,
}

impl SerialTasks {
    /// Request cooperative shutdown (does not wait).
    pub fn cancel(&self) {
        self.shutdown.cancel();
    }

    /// Stop tasks and ensure read/write halves are dropped (FD can be reopened).
    ///
    /// This method is idempotent: calling it multiple times (even from clones)
    /// will only stop/join the tasks once.
    #[instrument(skip(self), fields(id = %self.id))]
    pub async fn stop(&self) {
        self.shutdown.cancel();

        // Take and stop reader
        {
            let reader_jh = self.reader_jh.lock().await;
            reader_jh.abort();

            if !reader_jh.is_finished() {
                warn!("reader did not finish");
            }
        }

        // Take and stop writer
        {
            let writer_jh = self.writer_jh.lock().await;
            writer_jh.abort();

            if !writer_jh.is_finished() {
                warn!("writer jh did not finish");
            }
        }

        info!("stopped serial task");
    }
}

/// Spawn the serial reader+writer tasks.
///
/// - Reader: device -> ring + broadcast
/// - Writer: mpsc -> device (optionally echo writes into ring+broadcast)
#[instrument(skip_all, fields(id=id.serial_interface))]
pub fn spawn_io_tasks<R, W>(
    id: SerialID,
    mut reader: R,
    mut writer: W,
    sizes: BufferSizes,
    echo_writes: bool,
) -> SerialTasks
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
{
    let (write_tx, mut write_rx) = mpsc::channel::<Vec<u8>>(256);
    let (ws_tx, _ws_rx) = broadcast::channel::<Arc<[u8]>>(sizes.broadcast_buffer_size);
    let shutdown = CancellationToken::new();

    let state = SerialState {
        ring: Arc::new(RwLock::new(ByteRing::new(sizes.ring_buffer_size))),
        write_tx,
        ws_tx,
    };

    // Reader task
    {
        let st = state.clone();
        let shutdown_r = shutdown.clone();

        let reader_jh = tokio::spawn(async move {
            let mut buf = vec![0u8; 4096];

            loop {
                tokio::select! {
                    _ = shutdown_r.cancelled() => {
                        info!("serial reader stopping");
                        break;
                    }
                    res = reader.read(&mut buf) => {
                        match res {
                            Ok(0) => {
                                info!("serial reader EOF");
                                break;
                            }
                            Ok(n) => {
                                let chunk: Arc<[u8]> = Arc::from(&buf[..n]);
                                {
                                    let mut ring = st.ring.write().await;
                                    ring.push(&chunk);
                                }
                                let _ = st.ws_tx.send(chunk);
                            }
                            Err(e) => {
                                warn!(error=%e, "serial read failed");
                                break;
                            }
                        }
                    }
                }
            }
        });

        // Writer task
        let st = state.clone();
        let shutdown_w = shutdown.clone();

        let writer_jh = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_w.cancelled() => {
                        info!("serial writer stopping");
                        break;
                    }
                    msg = write_rx.recv() => {
                        match msg {
                            Some(data) => {
                                if let Err(e) = writer.write_all(&data).await {
                                    warn!(error=%e, "serial write failed");
                                    break;
                                }

                                if echo_writes {
                                    let chunk: Arc<[u8]> = Arc::from(data.into_boxed_slice());
                                    {
                                        let mut ring = st.ring.write().await;
                                        ring.push(&chunk);
                                    }
                                    let _ = st.ws_tx.send(chunk);
                                }
                            }
                            None => {
                                info!("write channel closed");
                                break;
                            }
                        }
                    }
                }
            }
        });

        let reader_jh = Arc::new(Mutex::new(reader_jh));
        let writer_jh = Arc::new(Mutex::new(writer_jh));
        let id = id.serial_interface;
        SerialTasks {
            id,
            shutdown,
            reader_jh,
            writer_jh,
            state,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::api::SerialID;
    use crate::serial::byte_ring::ByteRing;
    use crate::serial::serial_task::spawn_io_tasks;
    use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
    use tokio::time::{sleep, timeout};
    use tokio_test::io::{Builder, Mock};

    async fn wait_until_snapshot_starts_with(
        ring: &tokio::sync::RwLock<ByteRing>,
        prefix: &'static [u8],
    ) {
        // Poll ring until condition is met or timeout.
        timeout(Duration::from_secs(2), async {
            loop {
                let snap = ring.read().await.snapshot_all();
                if snap.starts_with(prefix) {
                    break;
                }
                sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("timeout waiting for ring snapshot");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn buffer_append_via_real_read_write_no_echo() {
        // Duplex simulates a TTY/device: bytes written by one side are readable by the other side.
        // We'll run a small echo loop on the "device" side so that writes reappear on the read side.
        let (svc_side, dev_side) = io::duplex(1024);

        // Give the service one side (split into reader/writer).
        let (reader, writer) = io::split(svc_side);

        // Spawn service tasks with echo_writes = false (what you want).
        let tasks = spawn_io_tasks(
            SerialID::from("testSerial".to_string()),
            reader,
            writer,
            Default::default(),
            false, // echo_writes disabled
        );
        let state = tasks.state.clone();

        // Subscribe to broadcasts produced by the reader task (device -> service).
        let mut rx = state.ws_tx.subscribe();
        let tx = state.write_tx.clone();

        // Device echo task: read what the service writes, write it back.
        let echo_jh = tokio::spawn(async move {
            let (mut dev_r, mut dev_w) = io::split(dev_side);
            let mut buf = [0u8; 256];
            loop {
                let n = match dev_r.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => n,
                    Err(_) => break,
                };
                if dev_w.write_all(&buf[..n]).await.is_err() {
                    break;
                }
            }
        });

        // Now: service writes -> device receives -> device echoes -> service reads -> ring updates.
        tx.send(b"Hello".to_vec()).await.unwrap();
        timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("timeout waiting for WS data")
            .expect("broadcast recv failed");

        tx.send(b" ".to_vec()).await.unwrap();
        timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("timeout waiting for WS data")
            .expect("broadcast recv failed");

        wait_until_snapshot_starts_with(&state.ring, b"Hello ").await;

        tx.send(b"World".to_vec()).await.unwrap();
        timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("timeout waiting for WS data")
            .expect("broadcast recv failed");

        wait_until_snapshot_starts_with(&state.ring, b"Hello World").await;

        // Cleanup: stop serial tasks + echo task
        tasks.stop().await;
        echo_jh.abort();
        let _ = echo_jh.await;
    }

    /// Spawns the serial tasks in echo mode so that writes are pushed into the ring buffer
    /// and broadcasted to WebSocket subscribers (used for test synchronization).
    fn spawn_echo_task(writer: Mock) -> crate::serial::serial_task::SerialTasks {
        // Reader is irrelevant for this test; we validate write->ring behavior via echo_writes=true
        let reader = tokio::io::empty();

        spawn_io_tasks(
            SerialID::from("testSerial".to_string()),
            reader,
            writer,
            Default::default(),
            true, // echo_writes = true => ring + broadcast are updated on writes
        )
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn buffer_append() {
        let writer = Builder::new()
            .write(b"Hello")
            .write(b" ")
            .write(b"World")
            .build();

        let tasks = spawn_echo_task(writer);
        let state = tasks.state.clone(); // likely Arc<SerialState>; adjust if needed

        // Subscribe before sending so we don't miss broadcast messages
        let mut rx = state.ws_tx.subscribe();
        let tx = state.write_tx.clone();

        tx.send(b"Hello".to_vec()).await.unwrap();
        timeout(Duration::from_secs(1), rx.recv())
            .await
            .unwrap()
            .unwrap();

        tx.send(b" ".to_vec()).await.unwrap();
        timeout(Duration::from_secs(1), rx.recv())
            .await
            .unwrap()
            .unwrap();

        let s1 = state.ring.read().await.snapshot_all();
        assert_eq!(&s1[..], b"Hello ");

        tx.send(b"World".to_vec()).await.unwrap();
        timeout(Duration::from_secs(1), rx.recv())
            .await
            .unwrap()
            .unwrap();

        let s2 = state.ring.read().await.snapshot_all();
        assert_eq!(&s2[..], b"Hello World");

        // Important: stop tasks to avoid leaking background tasks between tests
        tasks.stop().await;
    }
}
