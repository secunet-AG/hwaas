# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

_: {
  perSystem =
    { pkgs, ... }:
    let
      script = pkgs.writeShellScriptBin "lint-SPDX-license-header" ''
        set -euo pipefail
        ${pkgs.reuse}/bin/reuse lint
      '';
    in
    {
      apps.lint-SPDX-license-header = {
        type = "app";
        program = "${script}/bin/lint-SPDX-license-header";
      };
    };
}
