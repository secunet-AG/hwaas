# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

_: {
  perSystem = { config, pkgs, ... }: {
    apps.serve-docs = {
      type = "app";
      program = "${pkgs.writeShellScript "serve-docs" ''
        set -e
        DOCS_DIR="$(${pkgs.git}/bin/git rev-parse --show-toplevel)/documentation"
        cp --remove-destination --no-preserve=mode,ownership ${config.packages.hwaas-oas} "$DOCS_DIR/public/openapi.json"
        cd "$DOCS_DIR"
        ${pkgs.pnpm_10}/bin/pnpm i
        exec ${pkgs.pnpm_10}/bin/pnpm dev
      ''}";
    };
  };
}
