# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

_: {
  imports = [ ];

  perSystem =
    {
      pkgs,
      config,
      lib,
      ...
    }:
    {
      devShells.default = pkgs.mkShell {
        shellHook = ''
          ${config.pre-commit.shellHook}

          echo -e "\033[1;36m" # Cyan
          figlet "HWaaS dev shell"
          echo -e "\033[0m" # No Color

          # context api migrations are done manualy.
          repo_root="$(git rev-parse --show-toplevel)"
          export DATABASE_URL="file:$repo_root/components/contextapi/development.db";
        '';
        inputsFrom =
          builtins.attrValues (lib.filterAttrs (n: _: n != "default") config.devShells)
          ++ config.pre-commit.settings.enabledPackages;
        packages = with pkgs; [
          colmena
          figlet
          jq
          nodejs
          pnpm
          nixd
          marksman
          httpie
          markdownlint-cli2
          openapi-generator-cli
          ssh-to-age
          sops
          man-db
        ];
      };
    };
}
