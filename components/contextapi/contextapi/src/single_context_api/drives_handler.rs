// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use image_api::ImageHandler;
use image_api::sha256hash::Sha256Hash;

use std::path::PathBuf;
use tokio::fs;
use tracing::{error, warn};

/// Name of the drives store folder that uploaded images will be cloned into
const DRIVE_STORE: &str = "drives";

pub(crate) type DriveHash = Sha256Hash;

#[derive(Debug)]
pub(crate) enum CreateDriveError {
    /// the new random drive hash is not valid
    InvalidDriveHash,
    StoreNotAccessible,
    ImageNotFound,
    ImageCloneFailure,
}

/// create a new random [`DriveHash`] and initialize it via cloning the backing image
pub(crate) async fn handle_create_drive(
    cfg: ImageHandler,
    image_hash: String,
) -> Result<DriveHash, CreateDriveError> {
    // As drives are writable, calculating a real hash does not make much sense.
    // Also, drives are individual mutable clones of images - so simply copying the image hash
    // does not fulfill the requirement of a unique ID.
    // Hence, we create a new unique ID via random function.
    // The format however has to meet the remote-usb configuration schema.
    // This is why the hash has a defined length and is passed into a new [`Sha256Hash`].
    let result = hex::encode(rand::random::<[u8; 32]>());
    let drive_hash = Sha256Hash::new(result).map_err(|e| {
        error!(error = ?e, "could not create new drive id");
        CreateDriveError::InvalidDriveHash
    })?;

    // check if drives directory exist. If not create it.
    let store_path: PathBuf = cfg
        .get_subdir_in_image_store(DRIVE_STORE)
        .map_err(|error| {
            error!(
                ?error,
                "could not determine valid drive store path location"
            );
            CreateDriveError::StoreNotAccessible
        })?;
    if !store_path.is_dir() {
        fs::create_dir_all(store_path.as_path())
            .await
            .map_err(|e| {
                error!(error = ?e, "Could not create drive store path");
                CreateDriveError::StoreNotAccessible
            })?;
    }

    // Initialize the drive if file exists

    let src = cfg
        .resolve_image_path_from_hash(image_hash)
        .map_err(|_| CreateDriveError::ImageNotFound)?;
    let dst = store_path.join(drive_hash.clone());
    if !src.is_file() {
        return Err(CreateDriveError::ImageNotFound);
    }
    if dst.is_file() {
        warn!(drive_hash = %drive_hash, "going to overwrite drive");
    }
    fs::copy(src.as_path(), dst.as_path()).await.map_err(|e| {
        error!(
            ?src, ?dst, error = ?e,
            "Could not clone image into drive",
        );
        CreateDriveError::ImageCloneFailure
    })?;

    Ok(drive_hash)
}

#[derive(Debug)]
pub(crate) enum DeleteDriveError {
    /// The dirve is propably gone.
    /// At leas it was not found within the filesystem
    DriveNotFound,

    /// Deletion failed due to some io error
    DeletionFailed,
}

/// deletes a drive referenced by [`DriveHash`] without any checks
pub(crate) async fn handle_delete_drive(
    cfg: ImageHandler,
    drive_hash: &DriveHash,
) -> Result<(), DeleteDriveError> {
    let drive = cfg
        .get_subdir_in_image_store(DRIVE_STORE)
        .map_err(|error| {
            error!(
                ?error,
                "could not determine valid drive store path location"
            );
            DeleteDriveError::DeletionFailed
        })?
        .join(drive_hash);

    if !drive.is_file() {
        return Err(DeleteDriveError::DriveNotFound);
    }

    fs::remove_file(drive).await.map_err(|e| {
        error!(error = ?e, "could not remove drive");
        DeleteDriveError::DeletionFailed
    })
}
