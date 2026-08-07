-- SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
--
-- SPDX-License-Identifier: Apache-2.0

CREATE TABLE context_lifetimes (
    -- Uuid represented as TEXT in sqlite
    context_id BINARY NOT NULL PRIMARY KEY,
    created TIMESTAMP NOT NULL,
    timeout TIMESTAMP NOT NULL,
    FOREIGN KEY (context_id) REFERENCES contexts(id) ON DELETE CASCADE
)
