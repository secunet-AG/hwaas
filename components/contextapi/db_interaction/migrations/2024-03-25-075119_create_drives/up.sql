-- SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
--
-- SPDX-License-Identifier: Apache-2.0

CREATE TABLE drives(
    id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    context_id BINARY NOT NULL,
    UNIQUE(name, context_id),
    FOREIGN KEY(context_id) REFERENCES contexts(id) ON DELETE CASCADE
)
