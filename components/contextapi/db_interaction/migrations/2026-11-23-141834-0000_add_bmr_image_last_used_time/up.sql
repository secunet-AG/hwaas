-- SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
--
-- SPDX-License-Identifier: Apache-2.0

-- Nullable column to signal that a pre-existing metadata entry has never had this field filled in.
ALTER TABLE bmr_image_metadatas ADD COLUMN last_used_utc DATETIME;
