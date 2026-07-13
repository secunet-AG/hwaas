// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

pub(crate) mod context_management;
mod drives_api;
pub(crate) mod drives_handler;
pub(crate) mod get_router;
mod machines_api;
mod network_api;
pub(crate) mod remote_api;
mod websocket;

use aide::OperationIo;
use axum::async_trait;
use axum::extract::{FromRef, FromRequestParts, Path};
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use axum::RequestPartsExt;
use db_interaction::models::context_id::ContextIdBytes;
use reqwest::StatusCode;

use tokio::sync::mpsc::Sender;
use tracing::error;

use crate::app_state::{AppState, UnImplementedState};
use crate::{
    context_manager::{ContextAccessToken, ContextManagerMessage},
    path_params::PathParamsContextId,
};

pub(crate) use context_management::ContextManagementApiState;
pub(crate) use drives_api::DrivesApiState;
pub(crate) use machines_api::MachineApiState;
pub(crate) use network_api::NetworkApiState;

/// Intermediate value to obtain a [`GuardedContext`].
pub(crate) struct ContextManagerTx(Sender<ContextManagerMessage>);
impl FromRef<AppState> for ContextManagerTx {
    fn from_ref(input: &AppState) -> Self {
        Self(input.context_manager_tx.clone())
    }
}

/// Workaround necessary for building the Open API specification.
impl FromRef<UnImplementedState> for ContextManagerTx {
    fn from_ref(_input: &UnImplementedState) -> Self {
        unimplemented!(
            "UnImplementedState should not be used when serving requests. Use AppState instead!"
        )
    }
}
/// Contains a token providing exclusive access to a context.
///
/// This type implements from RequestParts thus making it
/// conveniently available to handlers.
#[derive(OperationIo)]
#[aide(input_with = "Path<PathParamsContextId>")]
pub struct GuardedContext(pub(crate) ContextAccessToken);

#[async_trait]
impl<S> FromRequestParts<S> for GuardedContext
where
    S: Send + Sync + Clone,
    ContextManagerTx: FromRef<S>,
{
    type Rejection = Response;
    #[tracing::instrument(skip_all)]
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path(PathParamsContextId { ctx_id }): Path<PathParamsContextId> =
            parts.extract().await.map_err(IntoResponse::into_response)?;
        let ContextManagerTx(ctx_manager_tx) = ContextManagerTx::from_ref(state);
        let (return_with, receiver) = tokio::sync::oneshot::channel();
        let _ = ctx_manager_tx
            .send(ContextManagerMessage::GetPermit {
                ctx_id: ContextIdBytes::from(ctx_id),
                return_with,
            })
            .await;
        match receiver.await {
            Ok(Some(context_access_token)) => Ok(GuardedContext(context_access_token)),
            Ok(None) => Err((
                StatusCode::NOT_FOUND,
                "The context was not found. Perhaps it has timed out?",
            )
                .into_response()),
            Err(e) => {
                // This can occur if the context timed out or a delete request was sent after we requested exclusive access, but before our request was processed by the context manager.
                error!(error.msg = %e, "error when attempting to obtain context access token for handler");
                Err((StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong").into_response())
            }
        }
    }
}
