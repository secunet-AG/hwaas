// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use anyhow::Context as _;
use axum::extract::FromRef;
use chrono::DateTime;
use chrono::Utc;
use db_interaction::connection::DbFacade;
use db_interaction::schema::{bmr_image_metadatas, bmr_image_tag_map, bmr_image_tags};
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

use crate::{filesystem::write_and_hash, image_api::ExtraImageStoreData, sha256hash::Sha256Hash};

/// Magic number to represent that a numeric ID has not been initialized yet.
// NOTE(hartan): This is a very unfortunate design decision that is more or less forced upon us by
// the `diesel` crate. It appears there is no trivial way to create a single struct that represents
// a full database table, while at the same time making the `id` column optional for user-facing
// operations. This means we cannot reasonably differ between objects with "valid" IDs (i.e. taken
// from the database) and objects with "invalid" IDs (i.e. created from scratch by users). This
// would make for nicer API and more directed error messages, though. In a prior job I've made good
// experiences with a generic ID type that looked something like this:
//
//     pub struct ID<T: 'static, U: 'static> {
//         /// Raw unique ID used to address an object in the database.
//         ///
//         /// This represents the tables primary key. The option is used to distinguish between objects
//         /// that exist in the database (`Some`) and objects that must be created in the database
//         /// (`None`).
//         #[serde(skip_deserializing)]
//         raw: Option<T>,
//         /// Phantom use of the owned datatype `U` to distinguish type instances.
//         #[serde(skip)]
//         _inner: PhantomData<U>,
//     }
//
// This was a breeze to implement with `sqlx`, but not with `diesel`. The trait system used by the
// latter is incredibly convoluted and the maintainers have a very strong opinion regarding the use
// of generics with diesel:
//
// - <https://github.com/diesel-rs/diesel/discussions/3880>
// - <https://github.com/diesel-rs/diesel/discussions/4821>
//
// After having wasted waaay to many hours trying to figure out this trait system, I hereby
// surrender.
const ID_I32_UNINITIALIZED: i32 = -1;

/// Name of the image store folder that uploaded images will be stored temporarily in.
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

/// The image metadata that can be requested for each uploaded image.
#[derive(
    Serialize,
    Deserialize,
    JsonSchema,
    diesel::Queryable,
    diesel::Selectable,
    diesel::Identifiable,
    diesel::Insertable,
    diesel::AsChangeset,
    Debug,
)]
#[diesel(
    table_name = bmr_image_metadatas,
    check_for_backend(diesel::sqlite::Sqlite),
    treat_none_as_null = true,
)]
pub struct ImageMetadata {
    /// Internal ID of the database entry
    id: i32,
    /// sha256 checksum of the full user image.
    pub sha256: String,
    /// Name under which the user uploaded/saved the image
    pub upload_name: String,
    /// Name under which the image is stored in the filesystem.
    ///
    /// The name is always relative to the store directory.
    pub file_name: String,
    /// The size of the image in bytes
    pub size_bytes: i64,
    /// The time when the image was first stored
    pub created_utc: DateTime<Utc>,
    /// Architecture the image was built for
    pub architecture: Option<String>,
}

/// A user-defined tag that can be attached to stored BMR images.
///
/// The `name` and `description` can hold arbitrary content defined by the user.
///
/// # Examples
///
/// Create a new tag and persist it in the database:
///
/// ```ignore
/// let new_tag = ImageTag::new("some name", Some("an optional description"));
/// let created_tag = ImageHandler::add_tag(new_tag).await.unwrap();
/// ```
///
/// Modify an existing tag:
///
/// ```ignore
/// let mut existing_tag = ImageHandler::list_tags().await.unwrap().first().unwrap();
/// existing_tag.name = "a different name";
/// let modified_tag = ImageHandler::modify_tag(existing_tag).await.unwrap();
/// ```

#[derive(
    Serialize,
    Deserialize,
    JsonSchema,
    Debug,
    PartialEq,
    diesel::Queryable,
    diesel::Selectable,
    diesel::Identifiable,
    diesel::AsChangeset,
)]
#[diesel(
    table_name = bmr_image_tags,
    check_for_backend(diesel::sqlite::Sqlite),
    treat_none_as_null = true
)]
pub struct ImageTag {
    /// Internal ID of the database entry
    id: i32,
    /// Human-readable name of the tag as shown in the UI.
    pub name: String,
    /// Human-readable description of what this tag represents.
    pub description: Option<String>,
}

impl ImageTag {
    /// Create a new tag for uploading into the database.
    pub fn new<N: Into<String>, D: Into<String>>(name: N, description: Option<D>) -> Self {
        Self {
            id: ID_I32_UNINITIALIZED,
            name: name.into(),
            description: description.map(|d| d.into()),
        }
    }
}

#[derive(
    diesel::Queryable, diesel::Identifiable, diesel::Selectable, diesel::Associations, Debug,
)]
#[diesel(
    table_name = bmr_image_tag_map,
    belongs_to(ImageMetadata, foreign_key = bmr_image_metadata_id),
    belongs_to(ImageTag, foreign_key = bmr_image_tag_id),
    primary_key(bmr_image_metadata_id, bmr_image_tag_id),
    check_for_backend(diesel::sqlite::Sqlite)
)]
pub struct ImageTagMap {
    #[diesel(column_name = "bmr_image_metadata_id")]
    image_metadata_id: i32,
    #[diesel(column_name = "bmr_image_tag_id")]
    image_tag_id: i32,
}

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
            .field("db_connection", &"[REDACTED]")
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
        let store = std::path::absolute(store_path.as_ref())
            .context("failed to canonicalize store path {:?}")?;
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

    pub fn resolve_image_path<P: AsRef<Path>>(&self, image: P) -> anyhow::Result<PathBuf> {
        let rel_image_path = image.as_ref();
        if rel_image_path.is_absolute() {
            anyhow::bail!(
                "refusing to resolve absolute image path {:?}",
                rel_image_path
            );
        }

        let full_image_path = std::path::absolute(self.store_path.join(rel_image_path))
            .context("failed to resolve absolute path for image location")?;

        if !full_image_path.starts_with(&self.store_path) {
            anyhow::bail!(
                "resolved image path {:?} points outside of image store directory {:?}",
                full_image_path,
                self.store_path
            );
        }

        Ok(self.store_path.join(rel_image_path))
    }

    /// List all images that are currently in the store
    #[tracing::instrument(skip(self))]
    pub async fn list_images(&self) -> Result<Vec<ImageMetadata>, ImageHandlerError> {
        self.db_connection
            .execute_on_current_thread(|con| bmr_image_metadatas::table.load(con))
            .await
            .map_err(|error| {
                error!(%error, "failed to load all defined images from database");
                ImageHandlerError::MetadataError
            })
    }

    /// Upload a new image to the database.
    #[tracing::instrument(skip(self, stream))]
    pub async fn add_image<S>(
        &self,
        stream: S,
        partial_metadata: ExtraImageStoreData,
    ) -> Result<ImageMetadata, ImageHandlerError>
    where
        S: AsyncRead,
    {
        let metadata = self.store_image(stream, partial_metadata).await?;

        let result = self
            .db_connection
            .execute_on_current_thread(|con| {
                use db_interaction::schema::bmr_image_metadatas::dsl::*;
                use diesel::prelude::*;

                diesel::insert_into(bmr_image_metadatas)
                    .values((
                        sha256.eq(&metadata.sha256),
                        upload_name.eq(&metadata.upload_name),
                        file_name.eq(&metadata.file_name),
                        size_bytes.eq(&metadata.size_bytes),
                        created_utc.eq(&metadata.created_utc),
                        architecture.eq(&metadata.architecture),
                    ))
                    .get_result(con)
            })
            .await;
        match result {
            Ok(val) => Ok(val),
            Err(error) => {
                error!(%error, "failed to store user image metadata in database");
                let image_file = self
                    .resolve_image_path(&metadata.file_name)
                    .map_err(|from| ImageHandlerError::StorageError {
                        from: std::io::Error::other(from),
                        details: "failed to resolve image file path to storage location",
                    })?;
                if let Err(e) = tokio::fs::remove_file(&image_file).await {
                    error!(error = %e, image = ?metadata, "failed to remove obsolete user image from disk");
                }
                Err(ImageHandlerError::MetadataError)
            }
        }
    }

    /// Modify an existing image in the database.
    ///
    /// Only the user-defined `upload_name` and stored image `architecture` can be modified after
    /// an image has been created. If any other metadata fields have been modified, the operation
    /// will fail. If you want to replace other metadata attributes, you must
    /// [delete](ImageHandler::remove_image) and [recreate](ImageHandler::add_image) the image from
    /// scratch.
    ///
    /// If you need to create a nonexistent image, use [`ImageHandler::add_image`].
    #[tracing::instrument]
    pub async fn modify_image(
        &self,
        image: ImageMetadata,
    ) -> Result<ImageMetadata, ImageHandlerError> {
        self.db_connection
            .execute_on_current_thread(|con| {
                use db_interaction::schema::bmr_image_metadatas::dsl::*;
                use diesel::prelude::*;

                diesel::update(
                    bmr_image_metadatas.filter(
                        // Filter for all the fields that users are *not* meant to modify to detect
                        // a) whether the user modified any of these fields, or b) the object in the
                        // DB has changed in the meantime.
                        id.eq(&image.id)
                            .and(sha256.eq(&image.sha256))
                            .and(file_name.eq(&image.file_name))
                            .and(size_bytes.eq(&image.size_bytes))
                            .and(created_utc.eq(&image.created_utc)),
                    ),
                )
                .set((
                    upload_name.eq(&image.upload_name),
                    architecture.eq(&image.architecture),
                ))
                .get_result(con)
            })
            .await
            .map_err(|e| {
                error!(error = %e, "failed to update existing BMR image metadata in the database");
                ImageHandlerError::MetadataError
            })
    }

    /// Remove an existing container image.
    ///
    /// This takes care both of deleting the metadata entry and the actual image blob residing on
    /// disk.
    #[tracing::instrument]
    pub async fn remove_image(&self, image: ImageMetadata) -> Result<(), ImageHandlerError> {
        let maybe_image_metadata = self
            .db_connection
            .execute_on_current_thread(|con| {
                use db_interaction::schema::bmr_image_metadatas::dsl::*;
                use diesel::prelude::*;

                // NOTE(hartan): The closure constrains us in a way that we cannot tell
                // whether failure is caused by the object not existing or something else. See:
                // <https://gitlab.cyberus-technology.de/cyberus/cidoka/hwaas/hwaas/-/work_items/51>
                diesel::delete(
                    // NOTE(hartan): Ignore all `Option<>` fields because they follow SQL
                    // semantics, so equality with `None` is always false. See:
                    // <https://docs.rs/diesel/2.3.10/diesel/expression_methods/trait.ExpressionMethods.html#method.eq>
                    bmr_image_metadatas.filter(
                        id.eq(&image.id())
                            .and(sha256.eq(&image.sha256))
                            .and(file_name.eq(&image.file_name))
                            .and(size_bytes.eq(&image.size_bytes))
                            .and(created_utc.eq(&image.created_utc)),
                    ),
                )
                .load::<ImageMetadata>(con)
            })
            .await
            .map_err(|e| {
                error!(error = %e, "failed to delete existing BMR image metadata in the database");
                ImageHandlerError::MetadataError
            })?;

        let metadata = match maybe_image_metadata.len() {
            0 => return Err(ImageHandlerError::ImageNotFound),
            1 => maybe_image_metadata.first().unwrap(),
            _ => return Err(ImageHandlerError::MetadataError),
        };

        let image_file = self
            .resolve_image_path(&metadata.file_name)
            .map_err(|from| ImageHandlerError::StorageError {
                from: std::io::Error::other(from),
                details: "failed to resolve image file path to storage location",
            })?;
        tokio::fs::remove_file(image_file)
            .await
            .map_err(|from| ImageHandlerError::StorageError {
                from,
                details: "failed to remove image blob for deleted metadata entry",
            })
    }

    /// List all tags currently known in the database.
    ///
    /// To create new tags, see [`ImageHandler::add_tag`]. To modify or remove existing tags, use
    /// [`ImageHandler::modify_tag`] or [`ImageHandler::remove_tag`].
    #[tracing::instrument(skip(self))]
    pub async fn list_tags(&self) -> Result<Vec<ImageTag>, ImageHandlerError> {
        self.db_connection
            .execute_on_current_thread(|con| bmr_image_tags::table.load(con))
            .await
            .map_err(|e| {
                error!(error = %e, "failed to load all defined image tags from database");
                ImageHandlerError::MetadataError
            })
    }

    /// Add a new tag to the database.
    ///
    /// Once created, it can be attached to existing images. If you need to update an existing tag,
    /// use [`ImageHandler::modify_tag`]. To remove a tag, see [`ImageHandler::remove_tag`].
    #[tracing::instrument]
    pub async fn add_tag(&self, tag: ImageTag) -> Result<ImageTag, ImageHandlerError> {
        use db_interaction::schema::bmr_image_tags::dsl::*;
        use diesel::ExpressionMethods;

        self.db_connection
            .execute_on_current_thread(|con| {
                diesel::insert_into(bmr_image_tags)
                    .values((name.eq(tag.name), description.eq(tag.description)))
                    .get_result(con)
            })
            .await
            .map_err(|e| {
                error!(error = %e, "failed to add new BMR image tag to the database");
                ImageHandlerError::MetadataError
            })
    }

    /// Modify an existing tag in the database.
    ///
    /// If you need to create a nonexistent tag, use [`ImageHandler::add_tag`].
    #[tracing::instrument]
    pub async fn modify_tag(&self, tag: ImageTag) -> Result<ImageTag, ImageHandlerError> {
        self.db_connection
            .execute_on_current_thread(|con| {
                use db_interaction::schema::bmr_image_tags::dsl::*;
                use diesel::prelude::*;

                diesel::update(bmr_image_tags.filter(id.eq(tag.id)))
                    .set(&tag)
                    .get_result(con)
            })
            .await
            .map_err(|e| {
                error!(error = %e, "failed to update existing BMR image tag in the database");
                ImageHandlerError::MetadataError
            })
    }

    /// Remove an existing tag from the database.
    #[tracing::instrument]
    pub async fn remove_tag(&self, tag: ImageTag) -> Result<(), ImageHandlerError> {
        let num_rows = self
            .db_connection
            .execute_on_current_thread(|con| {
                use db_interaction::schema::bmr_image_tags::dsl::*;
                use diesel::prelude::*;

                // NOTE(hartan): The closure constrains us in a way that we cannot tell
                // whether this is caused by the object not existing or something else. See:
                // <//gitlab.cyberus-technology.de/cyberus/cidoka/hwaas/hwaas/-/work_items/51>
                diesel::delete(
                    bmr_image_tags.filter(
                        id.eq(&tag.id)
                            .and(name.eq(&tag.name))
                            .and(description.eq(&tag.description)),
                    ),
                )
                .execute(con)
            })
            .await
            .map_err(|e| {
                error!(error = %e, "failed to update existing BMR image tag in the database");
                ImageHandlerError::MetadataError
            })?;

        debug_assert_eq!(num_rows, 1, "exactly one row should have been deleted");
        Ok(())
    }

    /// Get all tags currently attached to a particular image.
    #[tracing::instrument(skip(self))]
    pub async fn get_tags_for_image(
        &self,
        image: &ImageMetadata,
    ) -> Result<Vec<ImageTag>, ImageHandlerError> {
        self.db_connection
            .execute_on_current_thread(|con| {
                use diesel::prelude::*;

                ImageTagMap::belonging_to(image)
                    .inner_join(bmr_image_tags::table)
                    .select(ImageTag::as_select())
                    .load(con)
            })
            .await
            .map_err(|e| {
                error!(error = %e, "failed to update existing BMR image tag in the database");
                ImageHandlerError::MetadataError
            })?;

        todo!()
    }

    /// Get the metadata for the image that matches the given hash
    #[tracing::instrument(skip(self))]
    pub async fn get_image_by_hash(
        &self,
        image_hash: &Sha256Hash,
    ) -> Result<ImageMetadata, ImageHandlerError> {
        use diesel::prelude::*;

        self.db_connection
            .execute_on_current_thread(|con| {
                bmr_image_metadatas::table
                    .filter(bmr_image_metadatas::sha256.eq(&image_hash.0))
                    .select(ImageMetadata::as_select())
                    .get_result(con)
            })
            .await
            .map_err(|e| {
                error!(error = %e, "failed to query BMR image by sha256 hash");
                ImageHandlerError::ImageNotFound
            })
    }

    /// Store the given image along with the user defined image name and other metadata.
    pub async fn store_image<S>(
        &self,
        stream: S,
        metadata: ExtraImageStoreData,
    ) -> Result<ImageMetadata, ImageHandlerError>
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
            .map_err(|from| ImageHandlerError::StorageError {
                from,
                details: "cannot open temporary storage location",
            })?;

        let image_metadata = self
            .handle_image(stream, file, &tmp_storage_path, metadata.user_file_name)
            .await;

        // delete tmp image regardless of the success of handle_image
        let cleanup_result = remove_file(&tmp_storage_path).await.map_err(|io_err| {
            ImageHandlerError::StorageError {
                from: io_err,
                details: "cannot remove temporary image file",
            }
        });

        match (image_metadata, cleanup_result) {
            (Ok(ret), Ok(_)) => Ok(ret),
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
    ) -> Result<ImageMetadata, ImageHandlerError>
    where
        S: AsyncRead,
        P: AsRef<Path>,
    {
        let tmp_storage_path = path.as_ref();
        let (calculated_image_hash, image_size) =
            write_and_hash(stream, file).await.map_err(|io_err| {
                ImageHandlerError::StorageError {
                    from: io_err,
                    details: "failed to write and hash user image",
                }
            })?;

        let image_filename = format!("{}.bmrimg", calculated_image_hash);
        let target_image = self.resolve_image_path(&image_filename).map_err(|error| {
            ImageHandlerError::StorageError {
                from: std::io::Error::other(error),
                details: "failed to assemble final image store location",
            }
        })?;

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

        Ok(ImageMetadata {
            id: ID_I32_UNINITIALIZED,
            size_bytes: image_size.try_into().unwrap(),
            file_name: image_filename,
            created_utc: chrono::DateTime::from(std::time::SystemTime::now()),
            sha256: calculated_image_hash.0,
            upload_name: user_specified_image_name,
            architecture: None,
        })
    }
}
