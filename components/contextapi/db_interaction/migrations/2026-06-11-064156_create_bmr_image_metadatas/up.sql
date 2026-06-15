CREATE TABLE bmr_image_metadatas(
    -- Unique ID of a given image
    id INTEGER NOT NULL PRIMARY KEY,
    -- SHA256 checksum of the full image blob
    --
    -- This will break in the face of hash collisions, but I think those are
    -- unlikely to happen in our use case and easily fixed by deleting the
    -- colliding image.
    sha256 TEXT NOT NULL UNIQUE,
    -- Name the user chose while uploading the image
    upload_name TEXT NOT NULL,
    -- Name of the image residing on disk
    file_name TEXT NOT NULL,
    -- Size in bytes of the file representing the image
    size_bytes BIGINT NOT NULL,
    -- Time when the image was uploaded.
    --
    -- In an ideal world, this is equal to the last modification time of the
    -- image on disk. This may warrant a sanity check.
    --
    -- NOTE: We use 'DATETIME' instead of 'TIMESTAMP' because a) the latter
    -- will get into trouble in the year 2038 and b) the latter does magic
    -- timezone conversions based on server settings. Since Rust is
    -- well-capable of handling timezones, we let the application do this
    -- properly.
    -- See: <https://dev.mysql.com/doc/refman/8.4/en/datetime.html>
    created_utc DATETIME NOT NULL,

    -- Fields below should be nullable or have sane defaults for backwards
    -- compatibility.

    -- Platform architecture the image was compiled for. Must be given by the
    -- user.
    architecture TEXT DEFAULT NULL
    -- Tags are stored separately to ensure they are properly reusable
)
