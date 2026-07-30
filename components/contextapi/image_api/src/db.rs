//! # Image Handler Database Interactions
//!
//! This modules defines data types and utility functions to facilitate interaction between the
//! [`ImageHandler`] and the backing database that holds information about the actual images. You'll
//! probably find the following types particularly useful:
//!
//! - [`ImageMetadata`]
//! - [`ImageTag`]
//!
//! [`ImageHandler`]: crate::ImageHandler

use crate::sha256hash::Sha256Hash;
use db_interaction::models::bmr_image_metadata::{
    ImageMetadata as ModelImageMetadata, ImageTag as ModelImageTag,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Type alias for tag names.
pub type TagName = String;
/// Type alias for tag descriptions.
pub type TagDescription = Option<String>;

/// A tag attached to a BMR image.
#[derive(Debug, Clone, PartialEq, JsonSchema, Serialize, Deserialize)]
pub struct ImageTag {
    /// Concise name of a tag.
    pub name: TagName,
    /// Human-readable description of a tag.
    pub description: TagDescription,
}

impl ImageTag {
    pub fn new<N: Into<String>, D: Into<String>>(name: N, description: Option<D>) -> Self {
        Self {
            name: name.into(),
            description: description.map(|d| d.into()),
        }
    }
}

impl From<ModelImageTag> for ImageTag {
    fn from(value: ModelImageTag) -> Self {
        Self {
            name: value.name,
            description: value.description,
        }
    }
}

/// The ImageMetadata that can be requested for each uploaded image
// NOTE(hartan): Most of this type looks the way it does for compatibility with an earlier version
// of the REST API. Refer to the backing database structures
// ([`db_interaction::models::bmr_image_metadata`]) for a more polished representation.
#[derive(Debug, Clone, PartialEq, JsonSchema, Serialize, Deserialize)]
pub struct ImageMetadata {
    /// sha256 checksum of the full user image.
    pub sha256: Sha256Hash,
    /// The user specified file name of the image
    pub file_name: String,
    /// The size of the image in bytes
    pub size: u64,
    /// The time when the image was first stored
    // NOTE: `SystemTime` is chosen for compat with an earlier API version
    pub created: SystemTime,
    /// Compilation target architecture
    pub architecture: Option<String>,
    /// Arbitrary user-defined tags with extra information
    pub tags: Vec<ImageTag>,
}

/// failed to parse invalid database entry from column {column:?}: {cause:?}
#[derive(Debug, thiserror::Error, displaydoc::Display)]
pub struct InvalidEntry {
    column: &'static str,
    cause: String,
}

impl ImageMetadata {
    pub(crate) fn try_merge_from_db(
        metadata: ModelImageMetadata,
        tags: Vec<ModelImageTag>,
    ) -> Result<Self, InvalidEntry> {
        let sha256 = Sha256Hash::new(metadata.sha256).map_err(|error| InvalidEntry {
            column: "sha256",
            cause: error.to_string(),
        })?;
        let size = u64::try_from(metadata.size_bytes).map_err(|error| InvalidEntry {
            column: "size",
            cause: error.to_string(),
        })?;
        let tags = tags.into_iter().map(ImageTag::from).collect::<Vec<_>>();

        Ok(Self {
            sha256,
            file_name: metadata.file_name,
            size,
            created: metadata.created_utc.into(),
            architecture: metadata.architecture,
            tags,
        })
    }
}

impl TryFrom<(ModelImageMetadata, Vec<ModelImageTag>)> for ImageMetadata {
    type Error = InvalidEntry;

    fn try_from(value: (ModelImageMetadata, Vec<ModelImageTag>)) -> Result<Self, Self::Error> {
        Self::try_merge_from_db(value.0, value.1)
    }
}
