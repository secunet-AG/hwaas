// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

pub mod stdio;
pub mod tty;

use axum::async_trait;
use std::sync::{Arc, Mutex, RwLock};
use tokio::{
    sync::mpsc::{channel, Receiver, Sender},
    task::AbortHandle,
};

const MAX_BUFFER_SIZE: usize = 65536;

/// Generic interface that is implemented by all types of serial
#[async_trait]
pub trait SerialInput: Send {
    async fn read(&mut self) -> Result<Vec<u8>, std::io::Error>;
}

#[async_trait]
impl<I: SerialInput> SerialInput for Box<I> {
    async fn read(&mut self) -> Result<Vec<u8>, std::io::Error> {
        self.read().await
    }
}

/// Generic interface that is implemented by all types of serial
#[async_trait]
pub trait SerialOutput: Send {
    async fn write(&mut self, data: &[u8]) -> Result<(), std::io::Error>;
}

/// A thread-safe wrapper around any `SerialInput`/`SerialOutput` that
/// takes care of:
/// - Buffering read data
/// - Publishing read data to subscribers
#[derive(Clone)]
pub struct SerialState {
    buffer: Arc<RwLock<Vec<u8>>>,
    receivers: Arc<Mutex<Vec<Sender<Vec<u8>>>>>,
    sender: Sender<Vec<u8>>,
    /// Aborting the reader task on Drop quelches random panics after
    /// successful tests have run.
    #[allow(unused)] // for easier unit testing below
    reader_abort: Arc<Mutex<TaskAbortHandle>>,
    #[allow(unused)] // for easier unit testing below
    writer_abort: Arc<Mutex<TaskAbortHandle>>,
}

impl SerialState {
    /// Initializing `SerialState` depending on the serial type
    pub fn new<I, O>((mut input, mut output): (I, O)) -> Self
    where
        I: SerialInput + 'static,
        O: SerialOutput + 'static,
    {
        // Reading
        let buffer = Arc::new(RwLock::new(vec![]));
        let receivers = Arc::new(Mutex::new(Vec::<Sender<Vec<u8>>>::new()));
        let reader_abort = tokio::task::Builder::new()
            .name("serial reader")
            .spawn({
                let buffer = buffer.clone();
                let receivers = receivers.clone();
                async move {
                    while let Ok(data) = input.read().await {
                        tracing::trace!(data = ?data, "read");
                        {
                            let mut buffer = buffer.write().expect("buffer");
                            // Append
                            buffer.extend_from_slice(&data);
                            // Limit buffer size
                            let excess_bytes = buffer.len().saturating_sub(MAX_BUFFER_SIZE);
                            if excess_bytes > 0 {
                                *buffer = buffer.split_off(excess_bytes);
                            }
                        }
                        {
                            let mut receivers = receivers.lock().expect("receivers");
                            // Send synchronously to WebSocket subscribers so
                            // that those with full buffers are dropped immediately.
                            receivers.retain(|receiver| receiver.try_send(data.clone()).is_ok());
                        }
                    }
                    tracing::warn!("Serial EOF");
                }
            })
            .expect("serial read")
            .abort_handle();

        // Writing
        let (sender, mut receiver) = channel::<Vec<u8>>(1);
        let writer_abort = tokio::task::Builder::new()
            .name("serial_writer")
            .spawn(async move {
                while let Some(data) = receiver.recv().await {
                    output.write(&data[..]).await.unwrap();
                }
            })
            .expect("serial write")
            .abort_handle();

        SerialState {
            buffer,
            receivers,
            sender,
            reader_abort: Arc::new(Mutex::new(TaskAbortHandle(reader_abort))),
            writer_abort: Arc::new(Mutex::new(TaskAbortHandle(writer_abort))),
        }
    }

    /// Copy the read buffer
    pub fn get_buffer(&self) -> Vec<u8> {
        self.buffer.read().unwrap().clone()
    }

    /// Reset the read buffer
    pub fn clear_buffer(&self) {
        self.buffer.write().unwrap().clear();
    }

    /// Create a new receiver
    pub fn subscribe(&self) -> Receiver<Vec<u8>> {
        let (sender, receiver) = channel(32);
        self.receivers.lock().expect("receivers").push(sender);
        receiver
    }

    /// Write data
    pub async fn write(&self, data: Vec<u8>) -> Result<(), ()> {
        tracing::trace!(data = ?data, "write");
        self.sender.send(data).await.map_err(|_| ())
    }
}

/// Handle for a writer or reader abort
struct TaskAbortHandle(AbortHandle);

impl Drop for TaskAbortHandle {
    /// Enforce execution of abort functionality on drop
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoInput {
        receiver: Receiver<Vec<u8>>,
    }

    struct EchoOutput {
        sender: Sender<Vec<u8>>,
    }

    fn echo_state() -> SerialState {
        let (sender, receiver) = channel(1);
        let input = EchoInput { receiver };
        let output = EchoOutput { sender };
        SerialState::new((input, output))
    }

    #[async_trait]
    impl SerialInput for EchoInput {
        async fn read(&mut self) -> Result<Vec<u8>, std::io::Error> {
            Ok(self.receiver.recv().await.unwrap())
        }
    }

    #[async_trait]
    impl SerialOutput for EchoOutput {
        async fn write(&mut self, data: &[u8]) -> Result<(), std::io::Error> {
            self.sender.send(data.to_owned()).await.unwrap();
            Ok(())
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn buffer_append() {
        let state = echo_state();
        // Use the receiver just to synchronize on arrival of data
        let mut receiver = state.subscribe();
        state.write(b"Hello".to_vec()).await.unwrap();
        state.write(b" ".to_vec()).await.unwrap();
        receiver.recv().await;
        receiver.recv().await;
        assert_eq!(state.get_buffer(), b"Hello ".to_owned());
        state.write(b"World".to_vec()).await.unwrap();
        receiver.recv().await;
        assert_eq!(state.get_buffer(), b"Hello World".to_owned());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn buffer_clear() {
        let state = echo_state();
        assert_eq!(state.get_buffer(), b"".to_owned());
        // Use the receiver just to synchronize on arrival of data
        let mut receiver = state.subscribe();
        state.write(b"Hello".to_vec()).await.unwrap();
        receiver.recv().await;
        assert_eq!(state.get_buffer(), b"Hello".to_owned());
        state.clear_buffer();
        assert_eq!(state.get_buffer(), b"".to_owned());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn one_suscriber() {
        let state = echo_state();
        let mut receiver = state.subscribe();
        state.write(b"Hello".to_vec()).await.unwrap();
        assert_eq!(receiver.recv().await, Some(b"Hello".into()));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn two_suscribers() {
        let state = echo_state();
        let mut receiver1 = state.subscribe();
        let mut receiver2 = state.subscribe();
        state.write(b"Hello".to_vec()).await.unwrap();
        assert_eq!(receiver1.recv().await, Some(b"Hello".into()));
        assert_eq!(receiver2.recv().await, Some(b"Hello".into()));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn drop_slow_suscriber() {
        let state = echo_state();
        let mut receiver1 = state.subscribe();
        let mut receiver2 = state.subscribe();
        assert_eq!(state.receivers.lock().unwrap().len(), 2);

        // Let *only* receiver1 receive a lot of data
        const N: usize = 10_000;
        for _ in 0..N {
            state.write(b"Hello".to_vec()).await.unwrap();
            assert_eq!(receiver1.recv().await, Some(b"Hello".into()));
        }
        // receiver2 has been removed
        assert_eq!(state.receivers.lock().unwrap().len(), 1);

        // Let receiver2 receive
        let mut receiver2_received = 0;
        while let Some(data) = receiver2.recv().await {
            assert_eq!(data, b"Hello".to_owned());
            receiver2_received += 1;
        }
        assert!(receiver2_received < N);
    }
}
