# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

_: {
  perSystem =
    { config, pkgs, ... }:
    let
      script = pkgs.writeShellScriptBin "regen-expected-oas" ''
        cp ${config.packages.contextapi-oas} expected-oas/contextapi.openapi.json
        cp ${config.packages.net-ctrl-oas} expected-oas/net-ctrl.openapi.json
        cp -f ${config.packages.remote-hands-oas}/* expected-oas/
        cp ${config.packages.hwaas-oas} expected-oas/hwaas.openapi.json
      '';
    in
    {
      apps.regen-expected-oas = {
        type = "app";
        program = "${script}/bin/regen-expected-oas";
      };
    };
}
