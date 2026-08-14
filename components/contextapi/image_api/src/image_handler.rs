// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

//! # Handle storage of BMR images
//!
//! The [`ImageHandler`] manages storage space for user-provided BMR images, which are booted on
//! BMR targets during testing.
//!
//!
//! ## Implementation notes
//!
//! At present, uploaded images are stored on disk named after their content sha256 hash sum without
//! a file extension, **not after their `file_name`**. The latter is merely meant for users to
//! identify their images and need not be unique.
use anyhow::Context as _;
use axum::extract::FromRef;
use db_interaction::connection::DbFacade;
use db_interaction::models::bmr_image_metadata::{
    ImageMetadata as ModelImageMetadata, ImageTag as ModelImageTag, ImageTagMap as ModelImageTagMap,
};
use db_interaction::schema::{bmr_image_metadatas, bmr_image_tag_map, bmr_image_tags};
use diesel::RunQueryDsl;
use std::{
    fs::create_dir_all,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{
    fs::{File, OpenOptions, hard_link, remove_file},
    io::AsyncRead,
};
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::architectures::Architecture;
use crate::{filesystem::write_and_hash, image_api::ExtraImageStoreData, sha256hash::Sha256Hash};

pub use crate::db::{ImageMetadata, ImageTag, TagName};
pub use crate::image_file_path::ImageFilePath;

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

    /// the requested {0:?} wasn't found
    NotFound(&'static str),

    /// found multiple matching {0:?} where only a single match was expected
    OneExpected(&'static str),

    /// failed to process image metadata
    MetadataError,
}

impl axum::response::IntoResponse for ImageHandlerError {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;

        let code = match &self {
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::OneExpected(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let message = self.to_string();

        (code, message).into_response()
    }
}

/// Convenience function to create [`ImageHandlerError::StorageError`] variants.
fn to_storage_error(details: &'static str) -> impl FnOnce(std::io::Error) -> ImageHandlerError {
    move |from: std::io::Error| ImageHandlerError::StorageError { from, details }
}

/// Handle storage of BMR images.
///
///
/// ## Multiple Instances
///
/// It is perfectly fine to use multiple instances of the [`ImageHandler`] provided that no two
/// instances share the same image store path or database connection (
/// [Refer to the constructor](`ImageHandler::new`)). This is best ensured by either using only a
/// single [`ImageHandler`] in the first place, or by making sure that two instances always receive
/// fully distinct constructor arguments. When two [`ImageHandler`] instances share either of their
/// arguments, the behavior is undefined.
#[derive(Clone)]
pub struct ImageHandler {
    /// Path to the folder where the images should be stored
    store_path: PathBuf,
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
/// extraction for [`ImageHandler`] via FromRef is not
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

/// Supported operations for the [`ImageHandler::maintenance`] function.
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum MaintenanceOperations {
    /// Prune unused files from the image store directory.
    ///
    /// Subdirectories are ignored by this operation.
    PruneUnusedFiles,
}

impl MaintenanceOperations {
    pub fn all() -> Vec<Self> {
        vec![Self::PruneUnusedFiles]
    }
}

impl ImageHandler {
    /// Create a new ImageHandler that stores all images in the given folder.
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

        Ok(Self {
            store_path: store.to_path_buf(),
            db_connection,
        })
    }

    /// Run maintenance tasks in the image handler.
    ///
    /// If any maintenance task fails, the operation is aborted immediately i.e. any other requested
    /// maintenance tasks are skipped. The order in which maintenace tasks run is dictated by
    /// implementation details and may change without notice.
    ///
    /// To see available maintenance operations, refer to [MaintenanceOperations].
    pub async fn maintenance(&self, operations: Vec<MaintenanceOperations>) -> anyhow::Result<()> {
        let shared_error = Err(anyhow::anyhow!(
            "a maintenace tasks exited with an error, aborting maintenace job..."
        ));
        if operations.contains(&MaintenanceOperations::PruneUnusedFiles) {
            match self.maintenance_prune_unused_files().await {
                Ok(files) => {
                    info!(
                        ?files,
                        "{} unused files were pruned from the image store directory",
                        files.len()
                    );
                }
                Err(error) => {
                    let message = "failed to prune unused files from image store directory";
                    error!(?error, "{message}");
                    return shared_error.with_context(|| format!("{message}: {error}"));
                }
            }
        }

        Ok(())
    }

    /// Prune unused files from the image store directory.
    ///
    /// On success, a list of deleted files is returned. On failure, no such list is returned i.e.
    /// it's not possible to tell whether something has been deleted.
    async fn maintenance_prune_unused_files(&self) -> anyhow::Result<Vec<PathBuf>> {
        let images_in_db = self
            .list_image_metadatas()
            .await
            .context("failed to query known BMR images")?;

        // Files deleted from the image store.
        let mut removed_files = vec![];
        // List of image files covered by database metadata entries.
        let mut files_from_db = vec![];
        for image in images_in_db {
            let image_path = match self.resolve_image_path_from_hash(image.sha256.to_string()) {
                Ok(value) => value,
                Err(error) => {
                    let message = "failed to resolve backing file path from database metadata";
                    error!(?error, metadata = ?image, "{message}");
                    return Err(error).context(message);
                }
            };
            files_from_db.push(image_path);
        }

        let mut files_in_store = tokio::fs::read_dir(&self.store_path)
            .await
            .with_context(|| format!("failed to read files in {:?}", self.store_path))?;
        let err_entry = || {
            format!(
                "failed to read next directory entry im image store {:?}",
                self.store_path,
            )
        };

        // Check all files in the filesystem first
        while let Some(entry) = files_in_store.next_entry().await.with_context(err_entry)? {
            let raw_entry_path = entry.path();
            let err_context = || format!("failed to prune file {:?}", raw_entry_path);

            if !raw_entry_path.is_file() {
                debug!(entry = %raw_entry_path.display(), "skipping entry in image store since it's not a file");
                continue;
            }

            let entry_path = ImageFilePath::resolve(&self.store_path, &raw_entry_path)
                .context("failed to resolve image store entry as image path")
                .with_context(err_context)?;

            if files_from_db.iter().find(|p| *p == &entry_path).is_some() {
                debug!("skipping dir entry with backing database metadata");
                continue;
            }

            tokio::fs::remove_file(&entry_path).await.with_context(|| {
                format!("failed to remove file from image store {:?}", entry_path)
            })?;
            removed_files.push(entry_path.as_ref().to_owned());
        }

        Ok(removed_files)
    }

    /// Resolve an image hash to an absolute image store location, if possible.
    pub fn resolve_image_path_from_hash<H: TryInto<Sha256Hash> + std::fmt::Debug>(
        &self,
        hash: H,
    ) -> anyhow::Result<ImageFilePath> {
        let hash_dbg = format!("{:?}", hash);
        let maybe_hash = hash.try_into().map_err(|_| {
            anyhow::anyhow!(
                "cannot resolve image path from invalid sha256 hash: {}",
                hash_dbg
            )
        })?;
        ImageFilePath::resolve(&self.store_path, maybe_hash.0)
            .context("cannot resolve image file path")
    }

    /// Get the path to a subdirectory in the image store.
    ///
    /// Various checks are performed to ensure that the subdirectory doesn't collide with
    /// preexisting files or image paths that may be stored in the future.
    pub fn get_subdir_in_image_store<P: AsRef<Path>>(&self, subdir: P) -> anyhow::Result<PathBuf> {
        let subdir_path = subdir.as_ref();
        if subdir_path
            .file_name()
            .is_none_or(|f| f != subdir_path.as_os_str())
        {
            anyhow::bail!(
                "provided subdirectory {:?} must be a plain path component without nesting",
                subdir_path
            );
        }
        if subdir_path
            .to_str()
            // If it fails this, it's not a valid Unicode string and cannot be an ASCII hash either.
            .and_then(|s| Sha256Hash::new(s.to_string()).ok())
            .is_some()
        {
            anyhow::bail!("subdirectory must not be a sha256 hash sum");
        }

        let path = ImageFilePath::resolve(&self.store_path, subdir)
            .context("failed to generate valid subdirectory path in image store")?;
        if path.is_file() {
            anyhow::bail!("requested subdirectory path is already occupied by a file");
        }

        Ok(path.as_ref().to_owned())
    }

    /// List all images that are currently in the store
    #[tracing::instrument(skip(self))]
    pub async fn list_image_metadatas(&self) -> Result<Vec<ImageMetadata>, ImageHandlerError> {
        let (images, tags, mapping) = self
            .db_connection
            .execute_on_current_thread(|con| {
                use diesel::prelude::*;

                let raw_images = bmr_image_metadatas::table
                    .select(ModelImageMetadata::as_select())
                    .load(con)?;
                let raw_tags = bmr_image_tags::table
                    .select(ModelImageTag::as_select())
                    .load(con)?;
                let raw_image_tag_mapping = bmr_image_tag_map::table
                    .select(ModelImageTagMap::as_select())
                    .load(con)?;
                Ok((raw_images, raw_tags, raw_image_tag_mapping))

                //let tags = ModelImageTagMap::belonging_to(&raw_images)
                //    .inner_join(bmr_image_tags::table)
                //    // FIXME(hartan): This doesn't work as tuples (like used here) are represented
                //    // as `Record` in diesel, which is currently only implemented for the
                //    // `postgres_backend` feature. This could (maybe?) be worked around using
                //    // another custom type which fuses the image tag map and image tag into one, but
                //    // I have no idea whether that'll actually work. Also it means duplicating the
                //    // tag data structure, so we'd have to keep changes in sync in two places.
                //    .select((ModelImageTagMap::as_select(), ModelImageTag::as_select()))
                //    .load(con)?;
                //let tags_per_image = tags
                //    .grouped_by(&raw_images)
                //    .into_iter()
                //    .zip(raw_images)
                //    .map(|tags, image| ImageMetadata {})
                //    .collect::<Vec<_>>();
            })
            .await
            .map_err(|error| {
                error!(%error, "failed to load all defined BMR image metadata from database");
                ImageHandlerError::MetadataError
            })?;

        let mut result = Vec::with_capacity(images.len());
        for image in images {
            let tags_by_id = mapping
                .iter()
                .filter_map(|map| {
                    if map.metadata_id() == image.id() {
                        Some(map.tag_id())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            let image_tags = tags
                .iter()
                .filter(|tag| tags_by_id.contains(&tag.id()))
                .cloned()
                .collect::<Vec<_>>();

            result.push(
                ImageMetadata::try_merge_from_db(image, image_tags).map_err(|error| {
                    error!(%error, "failed to convert database response into valid image metadata");
                    ImageHandlerError::MetadataError
                })?,
            )
        }

        Ok(result)
    }

    /// Upload a new image to the database.
    ///
    /// If an image with the same sha256 hash previously existed, the previous image is kept. Only
    /// the creation time of the previous images metadata entry is updated with the current date and
    /// time to reflect the renewed "interest" in the image.
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
        if !metadata.tags.is_empty() {
            unimplemented!("attaching tags to images during initial upload");
        }

        let result = self
            .db_connection
            .execute_on_current_thread(|con| {
                use db_interaction::schema::bmr_image_metadatas::dsl::*;
                use diesel::prelude::*;

                let stored_image = diesel::insert_into(bmr_image_metadatas)
                    .values((
                        sha256.eq(&metadata.sha256.0),
                        file_name.eq(&metadata.file_name),
                        size_bytes.eq(i64::try_from(metadata.size)
                            .expect("image size should fit an i64 range")),
                        created_utc.eq(chrono::DateTime::<chrono::Utc>::from(metadata.created)),
                        architecture.eq(&metadata.architecture.map(|a| a.to_string())),
                    ))
                    .on_conflict(sha256)
                    .do_update()
                    .set((
                        created_utc.eq(chrono::DateTime::<chrono::Utc>::from(metadata.created)),
                        file_name.eq(&metadata.file_name),
                        architecture.eq(&metadata.architecture.map(|a| a.to_string())),
                    ))
                    .get_result::<ModelImageMetadata>(con)?;
                // TODO(hartan): Fill in tags from user upload in the distant future.
                let stored_tags = ModelImageTagMap::belonging_to(&stored_image)
                    .inner_join(bmr_image_tags::table)
                    .select(ModelImageTag::as_select())
                    .load::<ModelImageTag>(con)?;

                Ok((stored_image, stored_tags))
            })
            .await;
        match result {
            Ok((stored_image, stored_tags)) => {
                Ok(ImageMetadata::try_from((stored_image, stored_tags)).unwrap_or(metadata))
            }
            Err(error) => {
                error!(?error, "failed to store user image metadata in database");
                let image_file = self
                    .resolve_image_path_from_hash(metadata.sha256.to_string())
                    .map_err(|from| {
                        to_storage_error("failed to resolve image file path to storage location")(
                            std::io::Error::other(from),
                        )
                    })?;
                if let Err(e) = tokio::fs::remove_file(&image_file).await {
                    error!(error = %e, image = ?metadata, "failed to remove obsolete user image from disk");
                }
                Err(ImageHandlerError::MetadataError)
            }
        }
    }

    /// Modify the user filename for an existing image in the database.
    ///
    /// To modify the architecture of an image, please refer to
    /// [`modify_image_architecture`](ImageHandler::modify_image_architecture).
    /// To modify the tags of an image, please refer to the standalone operations for
    /// [adding](ImageHandler::add_tags_to_image) and
    /// [removing](ImageHandler::remove_tags_from_image) tags to/from images.
    ///
    /// If you need to create a nonexistent image, use [`ImageHandler::add_image`].
    #[tracing::instrument]
    pub async fn modify_image_file_name(
        &self,
        image: &Sha256Hash,
        new_file_name: String,
    ) -> Result<(), ImageHandlerError> {
        self.db_connection
            .execute_on_current_thread(|con| {
                use db_interaction::schema::bmr_image_metadatas::dsl::*;
                use diesel::prelude::*;

                diesel::update(bmr_image_metadatas.filter(sha256.eq(&image.0)))
                    .set(file_name.eq(&new_file_name))
                    .get_result::<ModelImageMetadata>(con)
            })
            .await
            .map_err(|e| {
                error!(error = %e, "failed to update existing BMR image metadata in the database");
                ImageHandlerError::MetadataError
            })?;
        Ok(())
    }

    /// Modify the architecture for an existing image in the database.
    ///
    /// To modify the user filename of an image, please refer to
    /// [`modify_image_file_name`](ImageHandler::modify_image_file_name).
    /// To modify the tags of an image, please refer to the standalone operations for
    /// [adding](ImageHandler::add_tags_to_image) and
    /// [removing](ImageHandler::remove_tags_from_image) tags to/from images.
    ///
    /// If you need to create a nonexistent image, use [`ImageHandler::add_image`].
    #[tracing::instrument]
    pub async fn modify_image_architecture(
        &self,
        image: &Sha256Hash,
        new_architecture: Option<Architecture>,
    ) -> Result<(), ImageHandlerError> {
        self.db_connection
            .execute_on_current_thread(|con| {
                use db_interaction::schema::bmr_image_metadatas::dsl::*;
                use diesel::prelude::*;

                diesel::update(bmr_image_metadatas.filter(sha256.eq(&image.0)))
                    .set(architecture.eq(&new_architecture.map(|a| a.to_string())))
                    .get_result::<ModelImageMetadata>(con)
            })
            .await
            .map_err(|e| {
                error!(error = %e, "failed to update existing BMR image metadata in the database");
                ImageHandlerError::MetadataError
            })?;
        Ok(())
    }

    /// Remove an existing container image.
    ///
    /// This takes care both of deleting the metadata entry and the actual image blob residing on
    /// disk.
    #[tracing::instrument]
    pub async fn remove_image(&self, image: &Sha256Hash) -> Result<(), ImageHandlerError> {
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
                    bmr_image_metadatas.filter(sha256.eq(&image.0)),
                )
                .load::<ModelImageMetadata>(con)
            })
            .await
            .map_err(|e| {
                error!(error = %e, "failed to delete existing BMR image metadata in the database");
                ImageHandlerError::MetadataError
            })?;

        let metadata = match maybe_image_metadata.len() {
            0 => return Err(ImageHandlerError::NotFound("BMR image")),
            1 => maybe_image_metadata.first().unwrap(),
            _ => return Err(ImageHandlerError::OneExpected("BMR images")),
        };

        let image_file = self
            .resolve_image_path_from_hash(metadata.sha256.to_string())
            .map_err(|from| {
                to_storage_error("failed to resolve image file path to storage location")(
                    std::io::Error::other(from),
                )
            })?;
        tokio::fs::remove_file(image_file)
            .await
            .map_err(to_storage_error(
                "failed to remove image blob for deleted metadata entry",
            ))
    }

    /// List all tags currently known in the database.
    ///
    /// To create new tags, see [`ImageHandler::add_tag`]. To modify or remove existing tags, use
    /// [`ImageHandler::modify_tag`] or [`ImageHandler::remove_tag`].
    #[tracing::instrument(skip(self))]
    pub async fn list_tags(&self) -> Result<Vec<ImageTag>, ImageHandlerError> {
        self.db_connection
            .execute_on_current_thread(|con| bmr_image_tags::table.load::<ModelImageTag>(con))
            .await
            .map_err(|e| {
                error!(error = %e, "failed to load all defined image tags from database");
                ImageHandlerError::MetadataError
            })
            .map(|tag_vec| tag_vec.into_iter().map(ImageTag::from).collect::<Vec<_>>())
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
                    .get_result::<ModelImageTag>(con)
            })
            .await
            .map_err(|e| {
                error!(error = %e, "failed to add new BMR image tag to the database");
                ImageHandlerError::MetadataError
            })
            .map(ImageTag::from)
    }

    /// Modify an existing tag in the database.
    ///
    /// The tag to modify is identified by its `name` property. If you need to create a nonexistent
    /// tag, use [`ImageHandler::add_tag`].
    #[tracing::instrument]
    pub async fn modify_tag(&self, tag: ImageTag) -> Result<ImageTag, ImageHandlerError> {
        let new_tag = self
            .db_connection
            .execute_on_current_thread(|con| {
                use db_interaction::schema::bmr_image_tags::dsl::*;
                use diesel::prelude::*;

                diesel::update(bmr_image_tags.filter(name.eq(tag.name)))
                    .set(description.eq(tag.description))
                    .get_result::<ModelImageTag>(con)
                    .optional()
            })
            .await
            .map_err(|e| {
                error!(error = %e, "failed to update existing BMR image tag in the database");
                ImageHandlerError::MetadataError
            })?;

        match new_tag {
            None => Err(ImageHandlerError::NotFound("BMR image tag")),
            Some(raw_tag) => Ok(ImageTag::from(raw_tag)),
        }
    }

    /// Remove an existing tag from the database.
    ///
    /// If the tag was attached to one or more BMR images, it will be removed from these BMR images,
    /// too.
    #[tracing::instrument]
    pub async fn remove_tag(&self, tag: ImageTag) -> Result<(), ImageHandlerError> {
        let amount = self
            .db_connection
            .execute_on_current_thread(|con| {
                use db_interaction::schema::bmr_image_tags::dsl::*;
                use diesel::prelude::*;

                // NOTE(hartan): The closure constrains us in a way that we cannot tell
                // whether this is caused by the object not existing or something else. See:
                // <https://gitlab.cyberus-technology.de/cyberus/cidoka/hwaas/hwaas/-/work_items/51>
                diesel::delete(
                    bmr_image_tags.filter(name.eq(&tag.name).and(description.eq(&tag.description))),
                )
                .execute(con)
            })
            .await
            .map_err(|e| {
                error!(error = %e, "failed to update existing BMR image tag in the database");
                ImageHandlerError::MetadataError
            })?;

        match amount {
            0 => Err(ImageHandlerError::NotFound("BMR image tag")),
            1 => Ok(()),
            _ => Err(ImageHandlerError::OneExpected("BMR image tags")),
        }
    }

    /// Add one or more tags to a BMR image.
    ///
    /// If a given tag name has no backing tag (i.e. a tag with that precise `name` property doesn't
    /// exist), it is silently ignored.
    ///
    /// Returns the number of tags added to the image.
    #[tracing::instrument(skip(self))]
    pub async fn add_tags_to_image<T: IntoIterator<Item = TagName> + std::fmt::Debug>(
        &self,
        tags: T,
        image: &Sha256Hash,
    ) -> Result<usize, ImageHandlerError> {
        use db_interaction::schema::bmr_image_tag_map::dsl::*;
        use diesel::prelude::*;

        let tag_names = tags.into_iter().collect::<Vec<_>>();

        self.db_connection
            .execute_on_current_thread(|con| {
                let image_id = bmr_image_metadatas::table
                    .filter(bmr_image_metadatas::sha256.eq(&image.0))
                    .select(bmr_image_metadatas::id)
                    .get_result::<i32>(con)?;
                let selected_tag_ids = bmr_image_tags::table
                    .filter(bmr_image_tags::name.eq_any(&tag_names))
                    .select(bmr_image_tags::id)
                    .load::<i32>(con)?;
                let insert = selected_tag_ids
                    .into_iter()
                    .map(|tag| (bmr_image_metadata_id.eq(image_id), bmr_image_tag_id.eq(tag)))
                    .collect::<Vec<_>>();
                diesel::insert_into(bmr_image_tag_map)
                    .values(insert)
                    .execute(con)
            })
            .await
            .map_err(|error| {
                error!(%error, ?image, "failed to add tags to BMR image");
                ImageHandlerError::MetadataError
            })
    }

    /// Remove one or more tags from an existing BMR image.
    ///
    /// Returns the number of tags deleted from the image.
    #[tracing::instrument(skip(self))]
    pub async fn remove_tags_from_image<T: IntoIterator<Item = TagName> + std::fmt::Debug>(
        &self,
        tags: T,
        image: &Sha256Hash,
    ) -> Result<usize, ImageHandlerError> {
        use db_interaction::schema::bmr_image_tag_map::dsl::*;
        use diesel::prelude::*;

        let tag_names = tags.into_iter().collect::<Vec<_>>();

        self.db_connection
            .execute_on_current_thread(|con| {
                let image_id = bmr_image_metadatas::table
                    .filter(bmr_image_metadatas::sha256.eq(&image.0))
                    .select(bmr_image_metadatas::id)
                    .get_result::<i32>(con)?;
                let selected_tag_ids = bmr_image_tags::table
                    .filter(bmr_image_tags::name.eq_any(&tag_names))
                    .select(bmr_image_tags::id)
                    .load::<i32>(con)?;

                let num_affected = diesel::delete(bmr_image_tag_map)
                    .filter(
                        bmr_image_metadata_id
                            .eq(image_id)
                            .and(bmr_image_tag_id.eq_any(selected_tag_ids)),
                    )
                    .execute(con)?;
                Ok(num_affected)
            })
            .await
            .map_err(|error| {
                error!(%error, ?image, "failed to add tags to BMR image");
                ImageHandlerError::MetadataError
            })
    }

    /// Get the metadata for the image that matches the given hash
    #[tracing::instrument(skip(self))]
    pub async fn get_image_metadata_by_hash(
        &self,
        image_hash: &Sha256Hash,
    ) -> Result<ImageMetadata, ImageHandlerError> {
        use diesel::prelude::*;

        self.db_connection
            .execute_on_current_thread(|con| {
                let metadata = bmr_image_metadatas::table
                    .filter(bmr_image_metadatas::sha256.eq(&image_hash.0))
                    .select(ModelImageMetadata::as_select())
                    .get_result(con)?;
                let stored_tags = ModelImageTagMap::belonging_to(&metadata)
                    .inner_join(bmr_image_tags::table)
                    .select(ModelImageTag::as_select())
                    .load::<ModelImageTag>(con)?;
                Ok((metadata, stored_tags))
            })
            .await
            .map_err(|e| {
                error!(error = %e, "failed to query BMR image by sha256 hash");
                ImageHandlerError::NotFound("BMR image")
            })
            .and_then(|(metadata, tags)| {
                ImageMetadata::try_merge_from_db(metadata, tags).map_err(|error| {
                    error!(%error, "failed to convert database response into valid image metadata");
                    ImageHandlerError::MetadataError
                })
            })
    }

    /// Store the given image along with the user defined image name and other metadata.
    async fn store_image<S>(
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
            .map_err(to_storage_error("cannot open temporary storage location"))?;

        let image_metadata = self
            .handle_image(stream, file, &tmp_storage_path, metadata.user_file_name)
            .await;

        // delete tmp image regardless of the success of handle_image
        let cleanup_result = remove_file(&tmp_storage_path)
            .await
            .map_err(to_storage_error("cannot remove temporary image file"));

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
        let (calculated_image_hash, image_size) = write_and_hash(stream, file)
            .await
            .map_err(to_storage_error("failed to write and hash user image"))?;

        let image_filename = &calculated_image_hash;
        let target_image = self
            .resolve_image_path_from_hash(image_filename.to_string())
            .map_err(|error| {
                to_storage_error("failed to assemble final image store location")(
                    std::io::Error::other(error),
                )
            })?;

        if target_image.is_file() {
            remove_file(&target_image).await.map_err(to_storage_error(
                "failed to remove previous image with identical name",
            ))?;
        }

        hard_link(&tmp_storage_path, &target_image)
            .await
            .map_err(to_storage_error(
                "cannot move image from temporary to permanent storage",
            ))?;
        Ok(ImageMetadata {
            sha256: calculated_image_hash,
            file_name: user_specified_image_name,
            size: image_size as u64,
            created: std::time::SystemTime::now(),
            architecture: None,
            tags: Vec::new(),
        })
    }
}
