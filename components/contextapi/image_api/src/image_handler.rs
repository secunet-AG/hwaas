// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use anyhow::Context as _;
use axum::extract::FromRef;
use chrono::DateTime;
use chrono::Utc;
use db_interaction::connection::DbFacade;
use db_interaction::schema::{bmr_image_metadatas, bmr_image_tags};
use diesel::RunQueryDsl;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    fs::create_dir_all,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{
    fs::{File, OpenOptions, hard_link, remove_file},
    io::AsyncRead,
};
use tracing::error;
use uuid::Uuid;

use crate::{
    db::ID, filesystem::write_and_hash, image_api::ExtraImageStoreData, sha256hash::Sha256Hash,
};

/// Name of the image store folder that uploaded images will be stored temporarily in
const UPLOAD_SUBDIR: &str = "uploads";

/// Enum containing all errors that can occur during image handling.
#[derive(Debug, thiserror::Error, displaydoc::Display)]
pub enum ImageHandlerError {
    /// failed to store user image: {details}
    StorageError {
        from: std::io::Error,
        details: &'static str,
    },

    /// no image with matching sha256 hash found
    ImageNotFound,

    /// failed to process image metadata
    MetadataError,
}

/// Specialized ID for manipulating images.
type ImageMetadataId = ID<i32, ImageMetadata>;

/// The image metadata that can be requested for each uploaded image.
#[derive(Serialize, Deserialize, JsonSchema, diesel::Queryable, diesel::Selectable, Debug)]
#[diesel(
    table_name = bmr_image_metadatas,
    check_for_backend(diesel::sqlite::Sqlite)
)]
pub struct ImageMetadata {
    /// Internal ID of the database entry
    id: ImageMetadataId,
    /// sha256 checksum of the full user image.
    sha256: String,
    /// Name under which the user uploaded/saved the image
    upload_name: String,
    /// Name under which the image is stored in the filesystem
    file_name: String,
    /// The size of the image in bytes
    size_bytes: i64,
    /// The time when the image was first stored
    created_utc: DateTime<Utc>,
    /// Architecture the image was built for
    architecture: Option<String>,
}

/// Specialized ID for manipulating image tags.
type ImageTagId = ID<i32, ImageTag>;


#[derive(Serialize, Deserialize, JsonSchema, Debug, diesel::Queryable, diesel::Selectable)]
#[diesel(
    table_name = bmr_image_tags,
    check_for_backend(diesel::sqlite::Sqlite)
)]
pub struct ImageTag {
    /// Internal ID of the database entry
    id: ImageTagId,
    /// Human-readable name of the tag as shown in the UI.
    name: String,
    /// Human-readable description of what this tag represents.
    description: Option<String>,
}

impl ImageTag {
    pub fn new<N: Into<String>, D: Into<String>>(name: N, description: Option<D>) -> Self {
        Self {
            id: ImageTagId::new_empty(),
            name: name.into(),
            description: description.map(|d| d.into()),
        }
    }
}

//#[derive(diesel::Queryable, diesel::Identifiable, diesel::Selectable, diesel::Associations, Debug)]
//#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
//#[diesel(
//    table_name = bmr_image_tag_map,
//    belongs_to(ImageMetadata),
//    belongs_to(ImageTag),
//    primary_key(bmr_image_metadata_id, bmr_image_tag_id)
//)]
//pub struct ImageTagMap {
//    #[diesel(column_name = "bmr_image_metadata_id")]
//    image_metadata_id: i32,
//    #[diesel(column_name = "bmr_image_tag_id")]
//    image_tag_id: i32,
//}

// NOTE(hartan): A valid SQL query to read this information (given an image ID) is:
//
//     SELECT id, name, description
//       FROM bmr_image_tag_map JOIN bmr_image_tags ON tag_id = id
//       WHERE image_id = 1;
//
// Now I just need to find a way to translate this to diesel. :(
//#[derive(Serialize, Deserialize, JsonSchema, diesel::Queryable)]
//pub struct ImageTagStore(pub Vec<ImageTag>);

/// Handles storage of boot images
#[derive(Clone)]
pub struct ImageHandler {
    /// Path to the folder where the images should be stored
    pub store_path: PathBuf,
    /// Database connection for metadata storage
    db_connection: Arc<DbFacade>,
}

// Required for the `axum` integration. Catching this here creates nicer error messages than the
// usual compile-time errors talking about unsatisfied trait bounds.
static_assertions::assert_impl_all!(ImageHandler: Send, Sync);

impl std::fmt::Debug for ImageHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImageHandler")
            .field("store_path", &self.store_path)
            .field("db_connection", &"CENSORED")
            .finish()
    }
}

/// If the ImageAPI is nested into a 'outer' API, the 'outer' state
/// has to implement this trait.
/// The outer state could not be known here and hence substate
/// extraction for [`ImageApiHandler`] via FromRef is not
/// implementable. But a generic implementation allows it for
/// any type implementing this trait.
pub trait IntoImageHandler {
    fn get_image_handler(&self) -> ImageHandler;
}

impl<S> FromRef<S> for ImageHandler
where
    S: IntoImageHandler + Send + Sync + Clone,
{
    fn from_ref(state: &S) -> Self {
        state.get_image_handler()
    }
}

impl ImageHandler {
    /// Create a new ImageHandler that stores all images in the given folder
    pub fn new<P>(store_path: P, db_connection: Arc<DbFacade>) -> Result<Self, anyhow::Error>
    where
        P: AsRef<Path>,
    {
        let store = store_path.as_ref();
        create_dir_all(store.join(UPLOAD_SUBDIR)).with_context(|| {
            format!(
                "failed to create folder structure for BMR image store at {:?}",
                store
            )
        })?;

        // TODO(hartan): I guess this is a good place to perform a metadata migration...

        Ok(Self {
            store_path: store.to_path_buf(),
            db_connection,
        })
    }

    /// List all images that are currently in the store
    pub async fn list_images(&self) -> Result<Vec<ImageMetadata>, ImageHandlerError> {
        // TODO(hartan): Query image table with diesel
        // TODO(hartan): Return the result
        let image = self
            .db_connection
            .spawn_call(|con| bmr_image_metadatas::table.load(con))
            .await
            .unwrap();
        Ok(image)
    }

    pub async fn list_tags(&self) -> Result<Vec<ImageTag>, ImageHandlerError> {
        self.db_connection
            .spawn_call(|con| bmr_image_tags::table.load(con))
            .await
            .map_err(|e| {
                error!(error = %e, "failed to load all defined image tags from database");
                ImageHandlerError::MetadataError
            })
    }

    #[tracing::instrument]
    pub async fn add_tag(&self, tag: ImageTag) -> Result<(), ImageHandlerError> {
        use db_interaction::schema::bmr_image_tags::dsl::*;
        use diesel::ExpressionMethods;

        self.db_connection
            .execute_on_current_thread(|con| {
                diesel::insert_into(bmr_image_tags)
                    .values((name.eq(tag.name), description.eq(tag.description)))
                    .execute(con)
            })
            .await
            .map_err(|e| {
                error!(error = %e, "failed to add new tag to the database");
                ImageHandlerError::MetadataError
            })
            .map(|_| ())
    }

    /// Get the metadata for the image that matches the given hash
    pub async fn get_image(
        &self,
        image_hash: &Sha256Hash,
    ) -> Result<ImageMetadata, ImageHandlerError> {
        // TODO(hartan): Get an image by hash, query the database
        todo!("obtaining image metadata");
    }

    /// Store the given image along with the specified image name.
    /// If the optional image_hash is provided, it will be checked against the contents of the
    /// image.
    pub async fn store_image<S>(
        &self,
        stream: S,
        metadata: ExtraImageStoreData,
    ) -> Result<String, ImageHandlerError>
    where
        S: AsyncRead,
    {
        // each upload is given their own UUID so even if the same image is uploaded multiple times
        // simultaneously, there can not be any file collisions
        let uuid = Uuid::new_v4().to_string();
        let tmp_storage_path = self.store_path.join(UPLOAD_SUBDIR).join(uuid);

        // Atomically create the temporary image if it does not exist.
        // this is reversed by the cleanup later
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp_storage_path)
            .await
            .map_err(|io_err| ImageHandlerError::StorageError {
                from: io_err,
                details: "cannot open temporary storage location",
            })?;

        let image_result = self
            .handle_image(stream, file, &tmp_storage_path, metadata.user_file_name)
            .await;

        // delete tmp image regardless of the success of handle_image
        let cleanup_result = remove_file(&tmp_storage_path).await.map_err(|io_err| {
            ImageHandlerError::StorageError {
                from: io_err,
                details: "cannot remove temporary image file",
            }
        });

        match (image_result, cleanup_result) {
            (Ok(hash), Ok(_)) => Ok(hash),
            (Err(image_err), Err(clean_err)) => {
                error!(error = %clean_err, "ignoring non-fatal error during image cleanup");
                Err(image_err)
            }
            (Err(image_err), Ok(_)) => Err(image_err),
            (Ok(_), Err(clean_err)) => Err(clean_err),
        }
    }

    /// Handle the storage of the given image
    async fn handle_image<S, P>(
        &self,
        stream: S,
        file: File,
        path: P,
        user_specified_image_name: String,
    ) -> Result<String, ImageHandlerError>
    where
        S: AsyncRead,
        P: AsRef<Path>,
    {
        let tmp_storage_path = path.as_ref();
        let calculated_image_hash = write_and_hash(stream, file).await.map_err(|io_err| {
            ImageHandlerError::StorageError {
                from: io_err,
                details: "failed to write and hash user image",
            }
        })?;

        let mut target_image = self.store_path.join(&calculated_image_hash);

        if target_image.is_file() {
            remove_file(&target_image)
                .await
                .map_err(|io_err| ImageHandlerError::StorageError {
                    from: io_err,
                    details: "failed to remove previous image with identical name",
                })?;
        }

        hard_link(&tmp_storage_path, &target_image)
            .await
            .map_err(|io_err| ImageHandlerError::StorageError {
                from: io_err,
                details: "cannot move image from temporary to permanent storage",
            })?;

        // TODO(hartan): Determine and store image metadata
        todo!("storing image metadata");
        //let meta_data = ImageMetadata {
        //    file_name: user_specified_image_name,
        //    size: todo!(),
        //    created: todo!(),
        //    architecture: todo!(),
        //    tags: todo!(),
        //};
        //if let Err(io_error) = write_meta_data(&target_image, meta_data).await {
        //        error!(
        //            "Error trying to write the metadata file {:?}: {io_error}",
        //            target_image,
        //        );
        //        return Err(ImageHandlerError::StorageError);
        //};

        Ok(calculated_image_hash)
    }
}
