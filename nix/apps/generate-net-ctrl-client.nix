# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

_: {
  perSystem =
    { config, pkgs, ... }:
    let
      script = pkgs.writeShellScriptBin "generate-net-ctrl-client" ''
        rsync -r --chmod=+w ${config.packages.net-ctrl-client}/ components/contextapi/net_ctrl_client
      '';
    in
    {
      apps.generate-net-ctrl-client = {
        type = "app";
        program = "${script}/bin/generate-net-ctrl-client";
      };
    };
}
