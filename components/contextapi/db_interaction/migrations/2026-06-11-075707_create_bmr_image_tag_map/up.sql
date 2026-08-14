-- SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
--
-- SPDX-License-Identifier: Apache-2.0

CREATE TABLE bmr_image_tag_map (
    -- Referenced unique ID of the BMR image
    bmr_image_metadata_id INTEGER NOT NULL,
    -- Referenced unique ID of the BMR image tag
    bmr_image_tag_id INTEGER NOT NULL,

    PRIMARY KEY (bmr_image_metadata_id, bmr_image_tag_id),
    FOREIGN KEY (bmr_image_metadata_id) REFERENCES bmr_image_metadatas(
        id
    ) ON DELETE CASCADE,
    FOREIGN KEY (bmr_image_tag_id) REFERENCES bmr_image_tags(
        id
    ) ON DELETE CASCADE
)
