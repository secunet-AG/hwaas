-- SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
--
-- SPDX-License-Identifier: Apache-2.0

CREATE TABLE switch_connections (
    id INTEGER NOT NULL PRIMARY KEY,
    interface TEXT NOT NULL,
    machine_id INTEGER NOT NULL,
    FOREIGN KEY (id) REFERENCES switch_ports(id) ON DELETE CASCADE,
    FOREIGN KEY (machine_id) REFERENCES machines(id) ON DELETE CASCADE,
    UNIQUE (interface, machine_id)
)
