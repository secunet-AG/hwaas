# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

_: {
  perSystem =
    { config, pkgs, ... }:
    {
      apps.ws-proxy-client = {
        type = "app";
        program = "${config.packages.ws-proxy-client}/bin/ws-proxy-client";
      };
    };
}
