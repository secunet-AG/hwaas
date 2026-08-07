// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::pin::Pin;

use aide::axum::routing::get_with;
use aide::axum::{ApiRouter, IntoApiResponse};
use aide::transform::{TransformOperation, TransformPathItem};
use axum::extract::{DefaultBodyLimit, State};
use axum::extract::{FromRef, Query};
use axum::extract::{Multipart, Path};
use axum::http::{Response, StatusCode};
use axum::response::IntoResponse as _;
use axum::Json;
use bytesize::ByteSize;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tower_http::limit::RequestBodyLimitLayer;
use tracing::{debug, instrument};

use crate::db::ImageMetadata;
use crate::image_handler::ImageHandler;
use crate::sha256hash::Sha256Hash;

/// The REST path parameter that identifies a specific boot image by its sha256 hash
#[derive(Deserialize, Serialize, JsonSchema)]
struct PathParamsImageHash {
    /// hash of a boot image (sha256sum)
    image_hash: Sha256Hash,
}

/// Takes the maximum allowed size for image uploads
///
/// # Returns
/// the correctly configured Axum router
pub fn get_image_api_router<S>(max_file_size: ByteSize) -> ApiRouter<S>
where
    ImageHandler: FromRef<S>,
    S: Send + Sync + Clone + 'static,
{
    ApiRouter::new()
        .api_route_with(
            "/",
            get_with(list_images, api_method_doc_list_images)
                .post_with(post_image, api_method_doc_upload_post),
            api_doc_image_api,
        )
        .api_route_with(
            "/:image_hash",
            get_with(status_image, api_method_doc_get_meta)
                .delete_with(delete_image, api_method_doc_delete),
            api_doc_image_api,
        )
        .layer(DefaultBodyLimit::disable())
        .layer(RequestBodyLimitLayer::new(max_file_size.as_u64() as usize))
}

fn api_doc_image_api(op: TransformPathItem) -> TransformPathItem {
    op.tag("Image API")
}

fn api_method_doc_list_images(op: TransformOperation) -> TransformOperation {
    op.description("Returns a list of all available images currently stored")
        .summary("list images")
        .response_with::<200, Json<ListImagesResponse>, _>(|op| {
            op.description("Return a dictionary containing the image hash and partial metadata including image size in bytes")
        })
        .response_with::<500, String, _>(|op| op.description("Internal error reason"))
}

fn api_method_doc_get_meta(op: TransformOperation) -> TransformOperation {
    op.description("Request metadata of an image")
        .summary("Get image metadata")
        .response_with::<200, Json<ImageMetadata>, _>(|op| op.description("metadata"))
        .response_with::<500, String, _>(|op| op.description("Internal error reason"))
}

fn api_method_doc_upload_post(op: TransformOperation) -> TransformOperation {
    op.description("Upload an image")
        .summary("Upload image")
        .response_with::<500, String, _>(|op| op.description("Internal error reason"))
        .response::<200, String>()
}

fn api_method_doc_delete(op: TransformOperation) -> TransformOperation {
    op.description("Mark an image for garbage collection")
        .summary("delete")
        .response_with::<202, String, _>(|op| op.description("Image marked for garbage collection"))
}

/// The ImageMetadata that can be requested for each uploaded image
#[derive(Serialize, JsonSchema)]
pub struct LegacyImageMetadata {
    /// The user specified file name of the image
    file_name: String,
    /// The size of the image in bytes
    size: u64,
    /// The time when the image was first stored
    created: std::time::SystemTime,
}

/// Response type for the [`list_images`] endpoint handler.
pub type ListImagesResponse = HashMap<String, LegacyImageMetadata>;

/// List all images currently stored.
///
/// # Returns
/// This handler returns a result.
/// The Ok value contains a HashMap with filenames and -sizes
/// On error a corresponding status code and message is returned.
#[instrument]
#[axum::debug_handler]
async fn list_images(
    State(image_handler): State<ImageHandler>,
) -> Result<Json<ListImagesResponse>, (StatusCode, String)> {
    let images = image_handler.list_image_metadatas().await.map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!(
                "Unexpected error occurred while trying to list all stored images: {:?}",
                err
            ),
        )
    })?;
    let response: ListImagesResponse = HashMap::from_iter(images.into_iter().map(|metadata| {
        (
            metadata.sha256.0,
            LegacyImageMetadata {
                file_name: metadata.file_name,
                size: metadata.size,
                created: metadata.created,
            },
        )
    }));
    Ok(Json::from(response))
}

/// This stub behaves like designed - it always returns 202 Accepted.
/// This signals the user that the image is marked for deletion.
/// TODO: impl mark the image for Garbage Collection (GC) and GC as such.
#[allow(clippy::let_with_type_underscore)]
#[instrument]
async fn delete_image(
    State(image_handler): State<ImageHandler>,
    Path(PathParamsImageHash { image_hash }): Path<PathParamsImageHash>,
) -> impl IntoApiResponse {
    (StatusCode::ACCEPTED, "Marked image for garbage collection")
}

#[instrument]
async fn status_image(
    State(image_handler): State<ImageHandler>,
    Path(PathParamsImageHash { image_hash }): Path<PathParamsImageHash>,
) -> Result<impl IntoApiResponse, Response<axum::body::Body>> {
    let meta_data = image_handler
        .get_image_metadata_by_hash(&image_hash)
        .await
        .map_err(|error| error.into_response())?;
    Ok(Json::from(meta_data))
}

/// Convert the given multipart request into a stream that contains the file of the first multipart
/// field. If the first multipart field does not contain a file, an error is returned.
async fn multipart_to_stream<'a>(
    multipart: &'a mut Multipart,
    compression: &Compression,
) -> Result<(Pin<Box<dyn tokio::io::AsyncRead + Send + 'a>>, String), (StatusCode, String)> {
    // NOTE(hartan): The way this is written makes it impossible to parse other multipart form
    // content. That is because the `field` being extracted here holds a mutable reference to the
    // `Multipart` itself, so getting other fields is not an option.
    let field = multipart
        .next_field()
        .await
        .map_err(|multi_error| {
            (
                StatusCode::BAD_REQUEST,
                format!(
                    "Encountered error while parsing field of multipart request: {multi_error}"
                ),
            )
        })?
        .ok_or((
            StatusCode::BAD_REQUEST,
            "Multipart request did not contain any fields".to_string(),
        ))?;
    let user_specified_name = field
        .file_name()
        .ok_or((
            StatusCode::BAD_REQUEST,
            "First field of multipart request was not a file".to_string(),
        ))?
        .to_owned();

    let body_with_io_error = futures::TryStreamExt::map_err(field, std::io::Error::other);
    let reader = tokio_util::io::StreamReader::new(body_with_io_error);
    let decompressed_field: Pin<Box<dyn tokio::io::AsyncRead + Send + 'a>> = match compression {
        Compression::None => Box::pin(reader),
        Compression::Zstd => Box::pin(async_compression::tokio::bufread::ZstdDecoder::new(reader)),
    };

    Ok((decompressed_field, user_specified_name))
}

#[derive(Debug, Deserialize, JsonSchema, Default)]
#[non_exhaustive]
#[serde(rename_all = "lowercase")]
pub enum Compression {
    /// No compression is applied to the image.
    #[default]
    None,
    /// The image is compressed using the `zstd` algorithm.
    Zstd,
}

/// Additional metadata to pass along with an uploaded image.
#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, default)]
pub struct ExtraImageStoreData {
    /// User-provided file name to identify the uploaded image by.
    pub user_file_name: String,
    /// Compression applied to the image before/during upload.
    pub compression: Compression,
}

/// Store the image that was uploaded via multipart request.
/// This request does not require the user to calculate the hash of his image beforehand.
/// On error the file is deleted at the end of the function call but may exist temporarily.
///
/// # Returns
/// An empty Ok on success and a status code and message on error.
#[instrument(skip(multipart))]
#[axum::debug_handler]
async fn post_image(
    State(image_handler): State<ImageHandler>,
    Query(mut metadata): Query<ExtraImageStoreData>,
    mut multipart: Multipart,
) -> Result<impl IntoApiResponse, Response<axum::body::Body>> {
    let (stream, user_specified_image_name) =
        multipart_to_stream(&mut multipart, &metadata.compression)
            .await
            .map_err(|error| error.into_response())?;
    if metadata.user_file_name.is_empty() {
        debug!("discarding empty user-provided image filename from query parameters");
        metadata.user_file_name = user_specified_image_name;
    }

    let metadata = image_handler
        .add_image(stream, metadata)
        .await
        .map_err(|error| error.into_response())?;
    // Maintain compatibility with earlier API versions
    Ok(metadata.sha256)
}
