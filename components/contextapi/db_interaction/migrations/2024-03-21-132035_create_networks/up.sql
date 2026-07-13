-- SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
--
-- SPDX-License-Identifier: Apache-2.0

CREATE TABLE networks(
    id SMALLINT NOT NULL PRIMARY KEY,
    context_id BINARY NOT NULL,
    name TEXT NOT NULL,
    UNIQUE(context_id, name),
    FOREIGN KEY(id) REFERENCES network_identifiers(id) ON DELETE CASCADE,
    FOREIGN KEY(context_id) REFERENCES contexts(id) ON DELETE CASCADE
)
