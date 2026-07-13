-- SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
--
-- SPDX-License-Identifier: Apache-2.0

CREATE TABLE enabled_ports(
    id INTEGER NOT NULL PRIMARY KEY,
    net_id SMALLINT NOT NULL,
    FOREIGN KEY(id) REFERENCES switch_ports(id) ON DELETE CASCADE,
    FOREIGN KEY(net_id) REFERENCES networks(id)
)
