// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

mod buffer;
mod list;
mod websocket;

use aide::{
    OperationIo,
    axum::{
        ApiRouter,
        routing::{get_with, post_with},
    },
    openapi::OpenApi,
    transform::TransformPathItem,
};
use axum::{
    RequestPartsExt, Router, async_trait,
    extract::{FromRequestParts, Path},
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;

use crate::app_state::AppState;
use crate::serial::serial_state::SerialState;
use buffer::{
    handle_delete_buffer, handle_delete_buffer_doc, handle_get_buffer, handle_get_buffer_doc,
    handle_post_buffer, handle_post_buffer_doc,
};
use list::{handle_get_all, handle_get_all_doc, handle_reset, handle_reset_doc};
use websocket::{handle_websocket, handle_websocket_doc};

#[derive(Clone, Deserialize, Serialize, JsonSchema, Debug)]
/// The ID in the API to specify the name of the serial interface to use.
pub struct SerialID {
    /// Name of the serial interface.
    pub(crate) serial_interface: String,
}

impl From<String> for SerialID {
    fn from(serial_interface: String) -> Self {
        Self { serial_interface }
    }
}

#[derive(OperationIo)]
#[aide(input_with = "Path<SerialID>")]
/// Struct with the `SerialState` retrieved by the given `SerialID`.
pub struct ExtractSerial(pub SerialID, pub SerialState);

/// Trait to allow all services that want to expose the serial API to have a
/// default interface to query for parts of the state.
/// Currently, this is `remote-serial`, as well as `remote-usb`.
#[async_trait]
pub trait HasSerial {
    /// Return the SerialState for the given serial_id.
    async fn get_serial(&self, id: &'_ str) -> Option<SerialState>;
    /// Return all known SerialStates.
    async fn get_serials(&self) -> Vec<SerialState>;
    /// Return a list of serial ids from the state.
    async fn get_serial_ids(&self) -> Vec<String>;
}

/// Implementation of the trait functions for `remote-serial`s `AppState`.
#[async_trait]
impl HasSerial for AppState {
    /// Return the SerialState for the given serial_id.
    async fn get_serial(&self, id: &'_ str) -> Option<SerialState> {
        self.serials.get(id).map(|s| s.state.clone())
    }
    /// Return all known SerialStates.
    async fn get_serials(&self) -> Vec<SerialState> {
        self.serials.values().map(|t| t.state.clone()).collect()
    }
    /// Return all known serial ids.
    async fn get_serial_ids(&self) -> Vec<String> {
        self.serials.keys().cloned().collect()
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for ExtractSerial
where
    S: Send + Sync + HasSerial,
{
    type Rejection = Response;

    /// Split the parts of the request and create structs as needed in the API handling.
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path(serial_id): Path<String> =
            parts.extract().await.map_err(IntoResponse::into_response)?;
        if let Some(serial) = state.get_serial(&serial_id).await {
            Ok(ExtractSerial(
                SerialID {
                    serial_interface: serial_id,
                },
                serial,
            ))
        } else {
            Err(StatusCode::NOT_FOUND.into_response())
        }
    }
}

/// Serial router with all sub routes.
pub fn serial_router<S>() -> ApiRouter<S>
where
    S: HasSerial + Clone + Send + Sync + 'static,
{
    ApiRouter::new()
        .api_route_with(
            "/serial",
            get_with(handle_get_all::<S>, handle_get_all_doc),
            serial_api_doc,
        )
        .api_route_with(
            "/serial/reset",
            post_with(handle_reset::<S>, handle_reset_doc),
            serial_api_doc,
        )
        .api_route_with(
            "/serial/:serial_interface",
            get_with(handle_get_buffer, handle_get_buffer_doc)
                .delete_with(handle_delete_buffer, handle_delete_buffer_doc)
                // PUT is handled for compatibility reasons
                // while POST actually has the right
                // semantics.
                .put_with(handle_post_buffer, handle_post_buffer_doc)
                .post_with(handle_post_buffer, handle_post_buffer_doc),
            serial_api_doc,
        )
        .api_route_with(
            "/serial/:serial_interface/websocket",
            get_with(handle_websocket, handle_websocket_doc),
            serial_api_doc,
        )
}

/// Group all serial API endpoints with the same tag.
fn serial_api_doc(op: TransformPathItem) -> TransformPathItem {
    op.tag("Serial API")
}

/// Create the router for the `SerialAPI`
pub async fn get_router<S>(state: AppState) -> Result<Router<S>, Infallible> {
    Ok(prepare_api_router(state).await?.0)
}

/// Build a `OpenAPI` based on the router implementation
pub async fn get_api<S>(state: AppState) -> Result<OpenApi, Infallible> {
    Ok(prepare_api_router::<S>(state).await?.1)
}

/// Prepare router with all sub routes.
pub async fn prepare_api_router<S>(state: AppState) -> Result<(Router<S>, OpenApi), Infallible> {
    let (router, api) = remote_axum::api_router(
        "remote-hands serial service",
        env!("CARGO_PKG_VERSION"),
        |router| async { Ok::<_, Infallible>(router.merge(serial_router())) },
    )
    .await?;
    let router = router.with_state(state);
    Ok((router, api))
}

#[cfg(test)]
mod tests {
    use axum_test::TestServer;
    use std::collections::HashMap;
    use std::time::Duration;
    use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
    use tokio::time::timeout;

    use crate::api::{SerialID, get_router};
    use crate::app_state::AppState;
    use crate::serial::serial_task::spawn_io_tasks;

    struct TestEnv {
        server: TestServer,
        id: SerialID,
        tasks: crate::serial::serial_task::SerialTasks,
        echo_jh: tokio::task::JoinHandle<()>,
    }

    /// Creates an in-memory echo serial backend.
    ///
    /// Returns:
    /// - reader
    /// - writer
    /// - JoinHandle of the echo task
    pub fn create_echo_serial(
        buffer_size: usize,
    ) -> (
        impl AsyncRead + Unpin + Send + 'static,
        impl AsyncWrite + Unpin + Send + 'static,
        tokio::task::JoinHandle<()>,
    ) {
        let (svc_side, dev_side) = io::duplex(buffer_size);

        // Echo task: whatever service writes -> gets written back
        let echo_jh = tokio::spawn(async move {
            let (mut r, mut w) = io::split(dev_side);
            let mut buf = vec![0u8; 4096];

            loop {
                let n = match r.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => n,
                    Err(_) => break,
                };

                if w.write_all(&buf[..n]).await.is_err() {
                    break;
                }
            }
        });

        let (reader, writer) = io::split(svc_side);

        (reader, writer, echo_jh)
    }

    async fn spawn_env() -> TestEnv {
        let id = SerialID::from("testSerial".to_string());
        let (reader, writer, echo_jh) = create_echo_serial(4096);

        let tasks = spawn_io_tasks(
            id.clone(),
            reader,
            writer,
            Default::default(),
            false, // echo_writes disabled
        );

        // Build real router
        let app_state = AppState::new(HashMap::from([(
            id.serial_interface.clone(),
            tasks.clone(),
        )]));

        let app = get_router::<()>(app_state).await.expect("Can't get router");

        let server = TestServer::builder()
            .http_transport()
            .build(app)
            .expect("Can't build server");

        TestEnv {
            server,
            id,
            tasks,
            echo_jh,
        }
    }

    async fn teardown(env: TestEnv) {
        env.tasks.stop().await;
        env.echo_jh.abort();
        let _ = env.echo_jh.await;
    }

    // -------------------------------------------------------------------------
    // two_suscribers()
    // -------------------------------------------------------------------------

    #[tokio::test(flavor = "multi_thread")]
    async fn two_subscribers() {
        let env = spawn_env().await;

        let get_ws = || async {
            env.server
                .get_websocket(&format!("/serial/{}/websocket", env.id.serial_interface))
                .await
                .into_websocket()
                .await
        };

        let mut ws1 = get_ws().await;
        let mut ws2 = get_ws().await;

        env.server
            .put(&format!("/serial/{}", env.id.serial_interface))
            .bytes("Hello".into())
            .await
            .assert_status_success();

        assert_eq!(ws1.receive_bytes().await, b"Hello".to_vec());
        assert_eq!(ws2.receive_bytes().await, b"Hello".to_vec());

        env.server
            .put(&format!("/serial/{}", env.id.serial_interface))
            .bytes("World".into())
            .await
            .assert_status_success();

        assert_eq!(ws1.receive_bytes().await, b"World".to_vec());
        assert_eq!(ws2.receive_bytes().await, b"World".to_vec());

        teardown(env).await;
    }

    // -------------------------------------------------------------------------
    // one_suscriber()
    // -------------------------------------------------------------------------

    #[tokio::test(flavor = "multi_thread")]
    async fn one_subscriber() {
        let env = spawn_env().await;

        let mut ws = env
            .server
            .get_websocket(&format!("/serial/{}/websocket", env.id.serial_interface))
            .await
            .into_websocket()
            .await;

        env.server
            .put(&format!("/serial/{}", env.id.serial_interface))
            .bytes("Hello".into())
            .await
            .assert_status_success();

        assert_eq!(ws.receive_bytes().await, b"Hello".to_vec());

        teardown(env).await;
    }

    // -------------------------------------------------------------------------
    // buffer_clear()
    // -------------------------------------------------------------------------

    #[tokio::test(flavor = "multi_thread")]
    async fn buffer_clear() {
        let env = spawn_env().await;

        env.server
            .put(&format!("/serial/{}", env.id.serial_interface))
            .bytes("HelloWorld".into())
            .await
            .assert_status_success();

        // wait briefly for echo + reader propagation
        timeout(Duration::from_secs(1), async {
            loop {
                let snap = env.tasks.state.ring.read().await.snapshot_all();
                if snap.starts_with(b"HelloWorld") {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();

        env.server
            .delete(&format!("/serial/{}", env.id.serial_interface))
            .await
            .assert_status_success();

        let snap = env.tasks.state.ring.read().await.snapshot_all();
        assert!(snap.is_empty());

        teardown(env).await;
    }

    // -------------------------------------------------------------------------
    // buffer_append()
    // -------------------------------------------------------------------------

    #[tokio::test(flavor = "multi_thread")]
    async fn buffer_append() {
        let env = spawn_env().await;

        env.server
            .put(&format!("/serial/{}", env.id.serial_interface))
            .bytes("Hello ".into())
            .await
            .assert_status_success();

        env.server
            .put(&format!("/serial/{}", env.id.serial_interface))
            .bytes("World".into())
            .await
            .assert_status_success();

        timeout(Duration::from_secs(1), async {
            loop {
                let snap = env.tasks.state.ring.read().await.snapshot_all();
                if snap.starts_with(b"Hello World") {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();

        teardown(env).await;
    }
}
