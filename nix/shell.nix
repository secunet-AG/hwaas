# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

_: {
  imports = [ ];

  perSystem =
    { pkgs
    , config
    , lib
    , ...
    }:
    {
      devShells.default = pkgs.mkShell {
        shellHook = ''
          ${config.pre-commit.installationScript}

          echo -e "\033[1;36m" # Cyan
          figlet "HWaaS dev shell"
          echo -e "\033[0m" # No Color

          # context api migrations are done manualy.
          export DATABASE_URL="file:$(git rev-parse --show-toplevel)/components/contextapi/development.db";
        '';
        inputsFrom = builtins.attrValues (lib.filterAttrs (n: _: n != "default") config.devShells);
        packages = with pkgs; [
          colmena
          figlet
          jq
          nodejs
          pnpm
          statix
          nixd
          marksman
          httpie
          markdownlint-cli2
          openapi-generator-cli
          ssh-to-age
          sops
          reuse
          man-db
        ];
      };
    };
}
