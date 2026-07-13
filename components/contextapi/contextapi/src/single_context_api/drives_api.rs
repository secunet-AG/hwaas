// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use crate::single_context_api::drives_handler::{
    handle_create_drive, handle_delete_drive, CreateDriveError, DeleteDriveError,
};
use aide::axum::IntoApiResponse;
use aide::{
    axum::{routing::get_with, ApiRouter},
    transform::{TransformOperation, TransformPathItem},
};
use axum::extract::FromRequestParts;
use axum::extract::{FromRef, State};
use axum::http::StatusCode;
use axum::{
    extract::{Path, Query},
    Json,
};

use db_interaction::connection::DbFacade;
use db_interaction::models::aliases::DriveId;
use db_interaction::models::context_id::ContextIdBytes;
use db_interaction::schema;
use diesel::prelude::*;
use error_utils::{log_err, log_then_replace_err};
use image_api::ImageHandler;
use image_api::IntoImageHandler;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;
use tracing::debug;
use tracing::error;
use tracing::error_span;

use super::drives_handler::DriveHash;
use super::GuardedContext;

#[derive(Clone)]
pub(crate) struct DrivesApiState {
    db_facade: Arc<DbFacade>,
    image_handler: ImageHandler,
}

impl DrivesApiState {
    pub(crate) fn new(db_facade: Arc<DbFacade>, image_handler: ImageHandler) -> Self {
        Self {
            db_facade,
            image_handler,
        }
    }
}

impl IntoImageHandler for DrivesApiState {
    fn get_image_handler(&self) -> ImageHandler {
        self.image_handler.clone()
    }
}

pub(crate) fn get_drives_router<S>() -> ApiRouter<S>
where
    S: Send + Sync + Clone + 'static,
    DrivesApiState: FromRef<S>,
    GuardedContext: FromRequestParts<S>,
{
    ApiRouter::new()
        .api_route_with(
            "/",
            get_with(get_drives, api_doc_get_drives_list),
            api_doc_machine_api,
        )
        .api_route_with(
            "/:drive_name",
            get_with(get_drive, api_doc_get_drive)
                .put_with(put_drive, api_doc_put_drive)
                .delete_with(delete_drive, api_doc_delete_drive),
            api_doc_machine_api,
        )
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct PathParamsDriveName {
    /// user's name of the drive
    drive_name: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct QueryParamsImageHash {
    /// id of the drive
    image_hash: String,
}

fn api_doc_machine_api(op: TransformPathItem) -> TransformPathItem {
    op.tag("Drives API")
}

fn api_doc_get_drives_list(op: TransformOperation) -> TransformOperation {
    op.description("Get the list of all allocated drives")
        .summary("list drives")
        .response_with::<200, Json<Vec<String>>, _>(|op| op.description("list of drives"))
}

fn api_doc_get_drive(op: TransformOperation) -> TransformOperation {
    op.description("Probe if a specified drive exists")
        .summary("get drive")
        .response_with::<200, (), _>(|op| op.description("drive exists"))
        .response_with::<404, String, _>(|op| op.description("drive does not exist"))
}
fn api_doc_put_drive(op: TransformOperation) -> TransformOperation {
    op.description("Create a new drive from a known image")
        .summary("create drive")
        .response::<200, ()>()
        .response_with::<500, String, _>(|op| op.description("drive creation failed internally"))
        .response_with::<409, String, _>(|op| op.description("drive exists"))
        .response_with::<422, String, _>(|op| op.description("base image not available"))
}

fn api_doc_delete_drive(op: TransformOperation) -> TransformOperation {
    op.description(
        "Delete a drive. \nPay attention: currently there is no \
        check if the drive is mounted at any machine!",
    )
    .summary("delete drive")
    .response::<200, ()>()
    .response_with::<410, String, _>(|op| op.description("Drive is already gone"))
    .response_with::<500, String, _>(|op| op.description("Drive allocation failed"))
    .response_with::<404, String, _>(|op| op.description("no such drive"))
}

/// This handler can be used to probe if a drive exists.
/// More or less this is only implemented for completeness reasons.
/// TODO: This could be used for downloading the drive and all its content
#[tracing::instrument(skip(dependencies))]
async fn get_drive(
    GuardedContext(context_access_token): GuardedContext,
    State(dependencies): State<DrivesApiState>,
    Path(PathParamsDriveName { drive_name }): Path<PathParamsDriveName>,
) -> Result<(), (StatusCode, &'static str)> {
    let ctx_id = context_access_token.context_id;
    if dependencies
        .db_facade
        .spawn_call(move |conn| lookup_drive_id(conn, ctx_id, &drive_name).optional())
        .await
        .map_err(log_then_replace_err!(
            "drive search failed",
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to lookup drive metadata from the database",
            )
        ))?
        .is_some()
    {
        Ok(())
    } else {
        Err((
            StatusCode::NOT_FOUND,
            "The drive was not found. Perhaps it has been deleted?",
        ))
    }
}

/// create a new drive
// TODO: Better naming id vs hash etc.
#[tracing::instrument(skip(dependencies))]
async fn put_drive(
    GuardedContext(context_access_token): GuardedContext,
    State(dependencies): State<DrivesApiState>,
    Path(PathParamsDriveName { drive_name }): Path<PathParamsDriveName>,
    Query(QueryParamsImageHash { image_hash }): Query<QueryParamsImageHash>,
) -> impl IntoApiResponse {
    // TODO: Consider avoiding the clone.
    let image_handler = dependencies.image_handler.clone();
    // Check if the drive already exists
    let ctx_id = context_access_token.context_id;
    {
        let drive_name = drive_name.clone();
        if dependencies
            .db_facade
            .spawn_call(move |conn| lookup_drive_id(conn, ctx_id, &drive_name).optional())
            .await
            .map_err(log_then_replace_err!(
                "drive search failed",
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to lookup drive metadata in the database",
                )
            ))?
            .is_some()
        {
            return Err((StatusCode::CONFLICT, "Drive already exists"));
        }
    }

    let hash = handle_create_drive(image_handler, image_hash)
        .await
        .map_err(|e| match e {
            CreateDriveError::InvalidDriveHash
            | CreateDriveError::StoreNotAccessible
            | CreateDriveError::ImageCloneFailure => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Unable to create drive")
            }
            CreateDriveError::ImageNotFound => {
                (StatusCode::UNPROCESSABLE_ENTITY, "Base image not available")
            }
        })?;

    debug!(
        drive_id = %hash,
        "drive created. Going to write its metadate to the database",
    );

    // Write the drive metadata (name, hash) to the database.
    let drive_metadata = db_interaction::models::drives::Drive {
        id: hash.0,
        name: drive_name,
        context_id: ctx_id,
    };
    // By passing on ownership of the context access token we can safely execute the
    // database query on a non-blocking thread pool.
    dependencies
        .db_facade
        .spawn_writing_call(move |conn| {
            let _context_access_guard = context_access_token;
            conn.immediate_transaction::<(), diesel::result::Error, _>(|conn| {
                drive_metadata
                    .insert_into(schema::drives::table)
                    .execute(conn)
                    .map(|_| ())
            })
        })
        .await
        .map_err(log_then_replace_err!(
            "drive insert transaction failed",
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Could not write drive metadata to the database",
            )
        ))?;

    Ok::<_, (StatusCode, &'static str)>(())
}

fn lookup_drive_id(
    conn: &mut SqliteConnection,
    ctx_id: ContextIdBytes,
    drive_name: &str,
) -> Result<DriveId, diesel::result::Error> {
    schema::drives::table
        .select(schema::drives::id)
        .filter(
            schema::drives::context_id
                .eq(ctx_id)
                .and(schema::drives::name.eq(drive_name)),
        )
        .first::<db_interaction::models::aliases::DriveId>(conn)
}

/// return a list of all drive names.
#[tracing::instrument(skip(dependencies))]
async fn get_drives(
    GuardedContext(context_access_token): GuardedContext,
    State(dependencies): State<DrivesApiState>,
) -> Result<Json<Value>, (StatusCode, &'static str)> {
    let ctx_id = context_access_token.context_id;
    let drive_names: Vec<String> = dependencies
        .db_facade
        .spawn_call(move |conn| {
            schema::drives::table
                .select(schema::drives::name)
                .filter(schema::drives::context_id.eq(ctx_id))
                .load(conn)
        })
        .await
        .map_err(log_then_replace_err!(
            "failed to load drives belonging to context",
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to load drive names from the database",
            )
        ))?;
    Ok(Json(json!(drive_names)))
}

/// Delete a drive.
// TODO: Better variable names.
#[tracing::instrument(skip(dependencies))]
async fn delete_drive(
    GuardedContext(context_access_token): GuardedContext,
    State(dependencies): State<DrivesApiState>,
    Path(PathParamsDriveName { drive_name }): Path<PathParamsDriveName>,
) -> Result<(), (StatusCode, &'static str)> {
    let ctx_id = context_access_token.context_id;
    // TODO: Consider avoiding the clone
    let image_handler = dependencies.image_handler.clone();
    let drive_name_clone = drive_name.clone();
    let drive_id: Option<DriveId> = dependencies
        .db_facade
        .spawn_call(move |conn| lookup_drive_id(conn, ctx_id, &drive_name_clone).optional())
        .await
        .map_err(log_then_replace_err!(
            "failed to load drive id from the database",
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Drive metadata database lookup failed",
            )
        ))?;

    let Some(drive_id) = drive_id else {
        return Err((
            StatusCode::NOT_FOUND,
            "The Drive was not found. Perhaps it has been deleted?",
        ));
    };

    let hash = DriveHash::new(drive_id.clone()).map_err(|e| {
        error!(
            drive_name,
            drive_id,
            error.msg = %e,
            "BUG: invalid drive hash loaded from database"
        );
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Unexpected error occurred. Please contact the HWaaS maintainers",
        )
    })?;

    handle_delete_drive(image_handler, &hash)
        .await
        .map_err(|e| match e {
            DeleteDriveError::DriveNotFound => (StatusCode::GONE, "Drive not found"),
            DeleteDriveError::DeletionFailed => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Drive deletion failed")
            }
        })?;

    // Note that we pass on ownership of the context access token in order to
    // safely execute the database update on the non-blocking thread pool.
    let drive_id_clone = drive_id.clone();
    dependencies.db_facade.spawn_writing_call(move |conn| {
        let _context_guard = context_access_token;
        let drive_id = drive_id_clone;
        // In the rare event that this runs to completion outside of the scope of the handler
        // we create a span and log that an error occurred.
        let span = error_span!("deleting_drive_metadata_from_db", drive_id);
        let _entered = span.enter();
        diesel::delete(schema::drives::table)
            .filter(schema::drives::id.eq(drive_id))
            .execute(conn)
            .inspect_err(log_err!(
                "failed to delete drive from the database"
            )).map(|_| ())
        }).await.inspect_err(|e| error!(drive_id, error.msg = %e,  "could not delete drive metadata: The maintainer may want to do this manually"))
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Could not delete drive metadata from the database"))
}
