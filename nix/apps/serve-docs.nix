# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

_: {
  perSystem =
    { config, pkgs, ... }:
    {
      apps.serve-docs = {
        type = "app";
        program = "${pkgs.writeShellScript "serve-docs" ''
          exec ${pkgs.python3}/bin/python3 -m http.server \
              --bind 127.0.0.1 \
              --directory ${config.packages.docs-html}
        ''}";
      };
    };
}
