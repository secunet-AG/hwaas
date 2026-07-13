-- SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
--
-- SPDX-License-Identifier: Apache-2.0

CREATE TABLE machine_reservations(
    id INTEGER NOT NULL PRIMARY KEY,
    context_id BINARY NOT NULL,
    machine_name TEXT NOT NULL,
    FOREIGN KEY(id) REFERENCES machines(id) ON DELETE CASCADE,
    FOREIGN KEY(context_id) REFERENCES contexts(id) ON DELETE CASCADE
)
