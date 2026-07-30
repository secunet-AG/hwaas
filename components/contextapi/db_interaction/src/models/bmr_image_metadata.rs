//! # BMR Image Metadata Handling
//!
//! This modules defines data types and utility functions to facilitate storage and processing of
//! additional metadata for BMR user images.

use crate::schema::{bmr_image_metadatas, bmr_image_tag_map, bmr_image_tags};

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
pub const ID_I32_UNINITIALIZED: i32 = -1;

/// The image metadata stored for each uploaded image.
#[derive(
    Debug,
    Clone,
    PartialEq,
    diesel::Queryable,
    diesel::Selectable,
    diesel::Identifiable,
    diesel::Insertable,
    diesel::AsChangeset,
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
    // NOTE: This is also used as actual filename to identify the image on disk after uploading.
    pub sha256: String,
    /// Name under which the user uploaded/saved the image
    pub file_name: String,
    /// The size of the image in bytes
    // NOTE(hartan): SQlite doesn't support unsigned long integers in diesel.
    pub size_bytes: i64,
    /// The time when the image was first stored
    pub created_utc: chrono::DateTime<chrono::Utc>,
    /// Architecture the image was built for
    pub architecture: Option<String>,
}

impl ImageMetadata {
    /// Get the raw, unique database ID representing this metadata block.
    pub fn id(&self) -> i32 {
        self.id
    }
}

/// A user-defined tag that can be attached to stored BMR images.
///
/// The `name` and `description` can hold arbitrary content defined by the user.
#[derive(
    Debug,
    Clone,
    PartialEq,
    diesel::Queryable,
    diesel::Selectable,
    diesel::Identifiable,
    diesel::Insertable,
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

    /// Get the raw, unique database ID representing this image tag.
    pub fn id(&self) -> i32 {
        self.id
    }
}

/// Mapping from BMR images to their attached tags.
///
/// This structure holds no information beyond a m:n mapping. It's only supporting code to enable
/// these queries.
#[derive(
    diesel::Queryable, diesel::Identifiable, diesel::Selectable, diesel::Associations, Debug, Clone,
)]
#[diesel(
    table_name = bmr_image_tag_map,
    belongs_to(ImageMetadata, foreign_key = bmr_image_metadata_id),
    belongs_to(ImageTag, foreign_key = bmr_image_tag_id),
    primary_key(bmr_image_metadata_id, bmr_image_tag_id),
    check_for_backend(diesel::sqlite::Sqlite)
)]
pub struct ImageTagMap {
    /// Image that is being mappend
    #[diesel(column_name = "bmr_image_metadata_id")]
    image_metadata_id: i32,
    /// Tag that is attached to the mapped image
    #[diesel(column_name = "bmr_image_tag_id")]
    image_tag_id: i32,
}

impl ImageTagMap {
    /// Get the image metadata database ID this mapping belongs to
    pub fn metadata_id(&self) -> i32 {
        self.image_metadata_id
    }

    /// Get the image tag database ID this mapping belongs to
    pub fn tag_id(&self) -> i32 {
        self.image_tag_id
    }
}
