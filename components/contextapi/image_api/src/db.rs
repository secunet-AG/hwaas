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
use chrono::{DateTime, Utc};
use db_interaction::schema::{bmr_image_metadatas, bmr_image_tag_map, bmr_image_tags};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
    Serialize,
    Deserialize,
    JsonSchema,
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
    pub created_utc: DateTime<Utc>,
    /// Architecture the image was built for
    pub architecture: Option<String>,
}

impl ImageMetadata {
    /// Create a new instance of `ImageMetadata` using [raw data](RawImageMetadata).
    ///
    /// The raw data isn't sanity checked against e.g. data residing on the filesystem. It's your
    /// responsibility to ensure the result is valid.
    pub(crate) fn new(raw: RawImageMetadata) -> Self {
        Self::from(raw)
    }

    /// Get the raw, unique database ID representing this metadata block.
    pub fn id(&self) -> i32 {
        self.id
    }
}

/// Collection of raw data needed to create [`ImageMetadata`].
pub struct RawImageMetadata {
    /// The hash value for the image.
    pub sha256: Sha256Hash,
    /// The user-provided name for identifying the image.
    ///
    /// **This field is optional** and the `sha256` value is assumed as default.
    pub file_name: Option<String>,
    /// Define the size (in bytes) of the uploaded image.
    pub size_bytes: ImageSize,
    /// The time at which the image was created.
    ///
    /// **This field is optional** and the current date and time is assumed as default.
    pub created_utc: Option<DateTime<Utc>>,
    /// The architecture for the uploaded image.
    ///
    /// **This field is optional** and an empty value is assumed as default.
    pub architecture: Option<String>,
}

impl From<RawImageMetadata> for ImageMetadata {
    fn from(value: RawImageMetadata) -> Self {
        Self {
            id: ID_I32_UNINITIALIZED,
            sha256: value.sha256.0.clone(),
            file_name: value.file_name.unwrap_or(value.sha256.0.clone()),
            size_bytes: value.size_bytes.0,
            created_utc: value
                .created_utc
                .unwrap_or(DateTime::<Utc>::from(std::time::SystemTime::now())),
            architecture: value.architecture,
        }
    }
}

/// Wrapper for user image upload sizes in bytes.
///
/// This performs sanity checks on the provided size values, i.e. that it doesn't overflow the
/// backing database storage type.
pub struct ImageSize(i64);

impl ImageSize {
    /// Create a new instance of [`ImageSize`].
    ///
    /// The provided value represents the size in bytes and must be larger than 0.
    pub fn new(value: i64) -> anyhow::Result<Self> {
        if value > 0 {
            Ok(Self(value))
        } else {
            anyhow::bail!(
                "user images must have a size larger than 0 bytes, got {}",
                value
            );
        }
    }
}

/// Blanket impl to allow easy conversions for some types.
impl<T> From<T> for ImageSize
where
    // If 'T' fits into a u32, it's probably fine for us as well.
    T: Into<i64> + Into<u32>,
{
    fn from(value: T) -> Self {
        Self(value.into())
    }
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
    /// Image that is being mappend
    #[diesel(column_name = "bmr_image_metadata_id")]
    image_metadata_id: i32,
    /// Tag that is attached to the mapped image
    #[diesel(column_name = "bmr_image_tag_id")]
    image_tag_id: i32,
}
