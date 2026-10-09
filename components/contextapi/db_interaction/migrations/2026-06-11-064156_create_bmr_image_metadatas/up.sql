-- SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
--
-- SPDX-License-Identifier: Apache-2.0

CREATE TABLE bmr_image_metadatas (
    -- Unique ID of a given image
    id INTEGER NOT NULL PRIMARY KEY,
    -- SHA256 checksum of the full image blob
    --
    -- This is also used as file name for permanently storing uploaded images.
    sha256 TEXT NOT NULL UNIQUE,
    -- Name the user chose while uploading the image
    --
    -- NOTE: This does not reflect the actual file name of the image file
    -- residing on disk after uploading.
    file_name TEXT NOT NULL,
    -- Size in bytes of the file representing the image
    size_bytes BIGINT NOT NULL,
    -- Time when the image was last uploaded.
    created_utc DATETIME NOT NULL,

    -- Fields below should be nullable or have sane defaults for backwards
    -- compatibility.

    -- Platform architecture the image was compiled for. Must be given by the
    -- user.
    architecture TEXT DEFAULT NULL
    -- Tags are stored separately to ensure they are properly reusable
)
