// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use crate::context_manager::ContextManagerMessage;
use crate::path_params::PathParamsContextId;
use crate::ContextApiConfig;
use aide::axum::routing::get_with;
use aide::axum::ApiRouter;
use aide::transform::TransformOperation;
use axum::extract::{FromRef, FromRequestParts, Json, Path, State};
use axum::http::StatusCode;
use chrono::Utc;
use db_interaction::connection::DbFacade;
use db_interaction::models::context_id::ContextIdBytes;
use db_interaction::models::contexts::ContextLifetime;
use db_interaction::schema::context_lifetimes;
use diesel::prelude::*;
use error_utils::log_then_replace_err;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use tokio::time::Duration;

use super::GuardedContext;

/// User-visible context information
#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub(crate) struct ContextInfo {
    /// Remaining seconds for the context
    pub lifetime: u64,
}

#[derive(Clone)]
pub(crate) struct ContextManagementApiState {
    config: Arc<ContextApiConfig>,
    db_facade: Arc<DbFacade>,
    ctx_manager_tx: Sender<ContextManagerMessage>,
}

impl ContextManagementApiState {
    pub(crate) fn new(
        config: Arc<ContextApiConfig>,
        db_facade: Arc<DbFacade>,
        ctx_manager_tx: Sender<ContextManagerMessage>,
    ) -> Self {
        Self {
            config,
            db_facade,
            ctx_manager_tx,
        }
    }
}
pub(crate) fn context_management_api_router<S>() -> ApiRouter<S>
where
    S: Send + Sync + Clone + 'static,
    ContextManagementApiState: FromRef<S>,
    GuardedContext: FromRequestParts<S>,
{
    ApiRouter::new().api_route(
        "/",
        get_with(handle_context_get, api_method_doc_get)
            .patch_with(handle_context_patch, api_method_doc_patch)
            .delete_with(handle_context_removal, api_method_doc_delete),
    )
}

fn api_method_doc_get(op: TransformOperation) -> TransformOperation {
    op.description("Get configuration of the context.")
        .summary("Get context configuration")
        .response_with::<200, Json<ContextInfo>, _>(|op| op.description("Context configuration"))
        .response_with::<404, String, _>(|op| op.description("The context was not found"))
        .response_with::<500, String, _>(|op| op.description("Something went wrong"))
}

#[tracing::instrument(skip_all)]
async fn handle_context_get(
    GuardedContext(context_access_token): GuardedContext,
    State(dependencies): State<ContextManagementApiState>,
) -> Result<Json<ContextInfo>, (StatusCode, &'static str)> {
    let ctx_id = context_access_token.context_id;
    let ctx_lifetime = get_context_lifetime(&dependencies.db_facade, ctx_id).await?;

    let timeout = ctx_lifetime.timeout.and_utc();
    let remaining = timeout - Utc::now();
    // If the timeout happens to be in the past the remaining time may be negative
    // in this case we want to simply say zero seconds as it will get terminated by
    // the context manager very soon
    let remaining_seconds = std::cmp::max(0, remaining.num_seconds()) as u64;
    Ok(Json(ContextInfo {
        lifetime: remaining_seconds,
    }))
}

fn api_method_doc_patch(op: TransformOperation) -> TransformOperation {
    op.description("Modify configuration of the context.")
        .summary("Modifies a context")
        .response::<202, String>()
        .response_with::<404, String, _>(|op| op.description("The context was not found"))
        .response_with::<409, String, _>(|op| {
            op.description("Context configuration not acceptable")
        })
        .response_with::<500, String, _>(|op| op.description("Something went wrong"))
}

/// Parameter changes desired by user in a PATCH request
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ContextPatch {
    /// Context lifetime timeout in seconds
    lifetime: Option<u64>,
}

#[tracing::instrument(skip_all)]
async fn handle_context_patch(
    GuardedContext(context_access_token): GuardedContext,
    State(dependencies): State<ContextManagementApiState>,
    Json(patch): Json<ContextPatch>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    let context_id = context_access_token.context_id;
    if let Some(lifetime) = patch.lifetime {
        let ctx_lifetime = get_context_lifetime(&dependencies.db_facade, context_id).await?;
        let lifetime = Duration::from_secs(lifetime);
        let new_timeout = Utc::now() + lifetime;
        if new_timeout
            > ctx_lifetime.created.and_utc()
                + Duration::from(dependencies.config.context_max_lifetime)
        {
            return Err((
                StatusCode::CONFLICT,
                "Cannot extend context lifetime timeout beyond maximum",
            ));
        }
        // Reserve space in the context manager communication's channel
        let send_permit =
            dependencies
                .ctx_manager_tx
                .reserve()
                .await
                .map_err(log_then_replace_err!(
                    "failed to reserve contex manager message",
                    (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong")
                ))?;

        // We write the update to the database. By passing on the context access token
        // we may safely do this on a non-blocking thread pool.
        dependencies
            .db_facade
            .spawn_writing_call(move |conn| {
                let _token_guard = context_access_token;
                diesel::update(context_lifetimes::table)
                    .filter(context_lifetimes::context_id.eq(context_id))
                    .set(context_lifetimes::timeout.eq(new_timeout.naive_utc()))
                    .execute(conn)
            })
            .await
            .map_err(log_then_replace_err!(
                "failed to update the context lifetime in the database",
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to update the database",
                )
            ))?;

        // Inform the context manager about the new timeout.
        send_permit.send(ContextManagerMessage::Timeout {
            ctx_id: context_id,
            timeout: lifetime,
        });
    }

    Ok(StatusCode::ACCEPTED)
}

#[tracing::instrument(skip_all)]
async fn get_context_lifetime(
    db_facade: &DbFacade,
    context_id: ContextIdBytes,
) -> Result<ContextLifetime, (StatusCode, &'static str)> {
    let user_facing_error = (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Could not load context information from the database",
    );
    db_facade
        .spawn_call(move |conn| {
            context_lifetimes::table
                .find(context_id)
                .select(ContextLifetime::as_select())
                .first(conn)
        })
        .await
        .map_err(log_then_replace_err!(
            "failed to read context lifetime",
            user_facing_error
        ))
}
fn api_method_doc_delete(op: TransformOperation) -> TransformOperation {
    op.description("Deletes the context and frees all of its assigned resources.")
        .summary("Deletes a context")
        .response::<202, String>()
        .response_with::<404, String, _>(|op| op.description("The context was not found"))
        .response_with::<500, String, _>(|op| op.description("Something went wrong"))
}

/// Delete the context identified by `ctx_id`. The side effects of this operation are
/// as follows:.
/// - All websockets connected to any network of the context is notified that it should shutdown.
/// - All switch ports of all machines in the context are disabled.
/// - All drives assigned to the context are deleted.
/// - All entries in the database dependent on the context id get deleted (happens automaticall for foreign table via ON DELETE CASCADE).
///
#[tracing::instrument(skip(dependencies))]
async fn handle_context_removal(
    State(dependencies): State<ContextManagementApiState>,
    Path(PathParamsContextId { ctx_id }): Path<PathParamsContextId>,
) -> StatusCode {
    // Note that we don't need a context access token if all we want is to inform the context manager that
    // we want the context to be deleted.
    let _ = dependencies
        .ctx_manager_tx
        .send(ContextManagerMessage::Delete(ctx_id.into()))
        .await;
    StatusCode::ACCEPTED
}
