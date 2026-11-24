-- SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
--
-- SPDX-License-Identifier: Apache-2.0

ALTER TABLE bmr_image_metadatas ADD COLUMN last_used_utc DATETIME NOT NULL default "1970-01-01T00:00:00";
