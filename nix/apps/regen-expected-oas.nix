# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

_: {
  perSystem =
    {
      config,
      pkgs,
      lib,
      ...
    }:
    let
      # Every '*-oas' package is copied to 'expected-oas/<name>.openapi.json',
      # mirroring the 'openapi-spec-*' golden tests in 'nix/tests/default.nix'.
      copy-commands = lib.pipe config.packages [
        (lib.filterAttrs (n: _v: (lib.match "^.*-oas$" n) != null))
        (lib.mapAttrsToList (
          name: drv: "cp ${drv} expected-oas/${lib.removeSuffix "-oas" name}.openapi.json"
        ))
        (lib.concatStringsSep "\n")
      ];
      script = pkgs.writeShellScriptBin "regen-expected-oas" ''
        set -euo pipefail
        ${copy-commands}
      '';
    in
    {
      apps.regen-expected-oas = {
        type = "app";
        program = "${script}/bin/regen-expected-oas";
        meta.description = "Regenerate expected OpenAPI spec";
      };
    };
}
