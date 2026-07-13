-- SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
--
-- SPDX-License-Identifier: Apache-2.0

CREATE TABLE switch_ports(
    id INTEGER NOT NULL PRIMARY KEY,
    switch TEXT NOT NULL,
    port TEXT NOT NULL,
    UNIQUE(switch, port)
)
