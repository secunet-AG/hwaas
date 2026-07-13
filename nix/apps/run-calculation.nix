# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

_: {
  perSystem =
    { config, pkgs, ... }:
    let
      input = ../../components/resource-calculator/calculations/hwaas24.yml;
      script = pkgs.writeShellScriptBin "run-calculation" ''
        ${config.packages.resource-calculator}/bin/resource-calculator ${input} --csv calculation.csv
      '';
    in
    {
      apps.run-calculation = {
        type = "app";
        program = "${script}/bin/run-calculation";
      };
    };
}
