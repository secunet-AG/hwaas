-- SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
--
-- SPDX-License-Identifier: Apache-2.0

CREATE TABLE machines(
    id INTEGER UNSIGNED NOT NULL PRIMARY KEY,
    platform TEXT NOT NULL,
    remote_usb TEXT NOT NULL,
    remote_power TEXT NOT NULL,
    remote_serial TEXT,
    remote_auxiliary TEXT,
    state SMALLINT UNSIGNED NOT NULL
);
