-- SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
--
-- SPDX-License-Identifier: Apache-2.0

CREATE TABLE machine_resets (
    id INTEGER NOT NULL PRIMARY KEY,
    started TIMESTAMP NOT NULL,
    FOREIGN KEY (id) REFERENCES machines(id) ON DELETE CASCADE
)
