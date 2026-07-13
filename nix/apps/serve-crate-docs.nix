# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ python3
, writeShellScript
, docs
,
}:
{
  type = "app";
  program = "${writeShellScript "serve-crate-docs" ''
    exec ${python3}/bin/python3 -m http.server \
        --bind 127.0.0.1 \
        --directory ${docs}
  ''}";
  meta.description = "server documentation localy";
}
