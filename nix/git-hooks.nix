# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

_: {
  perSystem =
    { pkgs
    , config
    , ...
    }:
    {
      treefmt.config = {
        projectRootFile = "flake.nix";
        programs = {
          nixpkgs-fmt = {
            enable = true;
          };
          shfmt.enable = true;
          yamlfmt.enable = true;
          taplo.enable = true;
          rustfmt.enable = false;
          prettier = {
            enable = true;
          };
        };
        settings = {
          formatter.prettier = {
            excludes = [
              "documentation/docs/spec/**"
              "documentation/docs/maintainers/secrets.md"
              "documentation/docs/maintainers/switch-models/aruba.md"
              "vue-client/pnpm-lock.yaml" # Usually good to exclude
              "**/*.yml"
              "**/*.yaml"
            ];
            options = [
              "--no-plugin-search"
            ];
          };
          global.excludes = [
            "^.cargo/.+$"
            "components/aruba-switch-mock/reference_schemars/**"
            "components/contextapi/net_ctrl_client/**"
            "**/*workspace-hack/Cargo.toml"
            "expected-oas/**"
            "vue-client/pnpm-lock.yaml"
            "*.svg"
            "*.img"
            "*.drawio"
          ];
        };

      };

      pre-commit = {
        check.enable = true;
        settings.hooks = {
          treefmt = {
            packageOverrides.treefmt = config.treefmt.build.wrapper;
            enable = true;
          };
          statix.enable = true;
          # deadnix.enable = true;
          reuse = {
            enable = true;
            package = pkgs.reuse;
          };
        };
      };
    };
}
