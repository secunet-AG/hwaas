# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

_: {
  imports = [ ];

  perSystem =
    { pkgs
    , config
    , ...
    }:
    {
      devShells.default = pkgs.mkShell {
        shellHook = ''
          ${config.pre-commit.installationScript}

          echo -e "\033[1;36m" # Cyan
          figlet "HWaaS dev shell"
          echo -e "\033[0m" # No Color

          # we can't use it for now as the reqwest-middleware dependency needs to be updated by openapi-generator-cli
          # https://github.com/OpenAPITools/openapi-generator/pull/20577
          #${config.apps.generate-net-ctrl-client.program}

          # context api migrations are done manualy.
          export DATABASE_URL="file:$(git rev-parse --show-toplevel)/components/contextapi/development.db";
        '';
        inputsFrom = [
          config.devShells.contextapi
        ];
        nativInputsFrom = [ ];
        packages = with pkgs; [
          colmena
          figlet
          rust-analyzer
          cargo-watch
          cargo-audit
          cargo-cyclonedx
          cargo-nextest
          jq
          nodejs
          pnpm
          statix
          nixd
          marksman
          httpie
          diesel-cli
          markdownlint-cli2
          openapi-generator-cli
          sqlite
          ssh-to-age
          sops
          reuse
        ];
      };
    };
}
