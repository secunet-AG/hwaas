// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use axum::extract::FromRef;
use futures::future::try_join_all;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::create_dir_all,
    io::ErrorKind,
    path::{Path, PathBuf},
    time::SystemTime,
};
use tokio::{
    fs::{hard_link, remove_file, write, File, OpenOptions},
    io::AsyncRead,
};
use tracing::log::error;
use uuid::Uuid;

use crate::{
    filesystem::{get_meta_data, list_files_of_directory, write_and_hash},
    image_api::ExtraImageStoreData,
    sha256hash::Sha256Hash,
};

/// Name of the image store folder that uploaded images will be stored temporarily in
const TMP: &str = "tmp";

/// The file extension used for the image metadata file
const META_EXT: &str = "txt";

/// Enum containing all errors that can occur during image handling.
#[derive(Debug)]
pub enum ImageHandlerError {
    /// General error for things that can go wrong while storing an image
    StorageError,

    /// Error if no image for the given hash exists
    ImageNotFound,

    /// General error for things that can go wrong while trying to access the metadata of an
    /// image
    MetadataError,
}

/// The ImageMetadata that can be requested for each uploaded image
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ImageMetadata {
    /// The user specified file name of the image
    file_name: String,
    /// The size of the image in bytes
    size: u64,
    /// The time when the image was first stored
    created: SystemTime,
}

/// Handles storage of boot images
#[derive(Clone, Debug)]
pub struct ImageHandler {
    /// Path to the folder where the images should be stored
    pub store_path: PathBuf,
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
    #[allow(clippy::result_unit_err)] // clippy does not like unit errors
    pub fn new<P>(store_path: P) -> Result<Self, ()>
    where
        P: AsRef<std::path::Path>,
    {
        let store = store_path.as_ref();
        create_dir_all(store.join(TMP)).map_err(|io_err| {
            error!("Error occurred while trying to create the image store folder: {io_err}");
        })?;
        Ok(Self {
            store_path: store.to_path_buf(),
        })
    }

    /// List all images that are currently in the store
    pub async fn list_images(&self) -> Result<HashMap<String, ImageMetadata>, ImageHandlerError> {
        let images = list_files_of_directory(&self.store_path, None)
            .await
            .map_err(|io_err| {
                error!("Error occurred while trying to list all files of directory: {io_err}");
                ImageHandlerError::StorageError
            })?;

        try_join_all(images.iter().map(|image_hash| async {
            let hash = Sha256Hash(image_hash.to_string());
            self.get_image_metadata(&hash)
                .await
                .map(|meta| (image_hash.clone(), meta))
        }))
        .await
        .map(|vec| vec.into_iter().collect())
    }

    /// Get the metadata for the image that matches the given hash
    pub async fn get_image_metadata(
        &self,
        image_hash: &Sha256Hash,
    ) -> Result<ImageMetadata, ImageHandlerError> {
        let (file_name, size, created) = get_meta_data(self.store_path.join(image_hash))
            .await
            .map_err(|io_err| match io_err.kind() {
                ErrorKind::NotFound => ImageHandlerError::ImageNotFound,
                _ => {
                    error!("Unexpected error occurred while creating metadata for image: {io_err}");
                    ImageHandlerError::MetadataError
                }
            })?;

        Ok(ImageMetadata {
            file_name,
            size,
            created,
        })
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
        let tmp_storage_path = self.store_path.join(TMP).join(uuid);

        // Atomically create the temporary image if it does not exist.
        // this is reversed by the cleanup later
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp_storage_path)
            .await
            .map_err(|io_err| {
                error!("Error occurred trying to create new image: {io_err}");
                ImageHandlerError::StorageError
            })?;

        let image_result = self
            .handle_image(stream, file, &tmp_storage_path, metadata.user_file_name)
            .await;

        // delete tmp image regardless of the success of handle_image
        let cleanup_result = remove_file(&tmp_storage_path).await.map_err(|io_err| {
            error!(
                "Error occurred trying to remove image: {:?} {io_err}",
                tmp_storage_path
            )
        });

        match (image_result, cleanup_result) {
            (Ok(hash), Ok(_)) => Ok(hash),
            (Err(e), _) => Err(e),
            _ => Err(ImageHandlerError::StorageError),
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
            error!("Error occurred trying to write and hash the image: {io_err}");
            ImageHandlerError::StorageError
        })?;

        let mut target_image = self.store_path.join(&calculated_image_hash);

        if target_image.is_file() {
            remove_file(&target_image).await.map_err(|io_err| {
                error!("Error occurred trying to remove previous image with same name: {io_err}");
                ImageHandlerError::StorageError
            })?;
        }

        hard_link(&tmp_storage_path, &target_image)
            .await
            .map_err(|io_err| {
            error!(
                "Error trying to move image {:?} from temporary storage ({:?}) to permanent storage ({:?}): {io_err}",
                target_image, tmp_storage_path, target_image
            );
            ImageHandlerError::StorageError
            })?;

        // the meta file is used to store additional information for the image such as the user
        // specified image name.
        // The metadata is overridden if another user uploads the same image.
        // This behavior might be confusing for the user but as the metadata only contains non
        // critical information, we are fine with this for the current MVP implementation.
        target_image.set_extension(META_EXT);
        write(&target_image, user_specified_image_name)
            .await
            .map_err(|io_error| {
                error!(
                    "Error trying to write the metadata file {:?}: {io_error}",
                    target_image,
                );
                ImageHandlerError::StorageError
            })?;

        Ok(calculated_image_hash)
    }
}
