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
              "vue-client/pnpm-lock.yaml"
              "**/*.yml"
              "**/*.yaml"
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
          statix = {
            enable = true;
            settings.ignore = [
              "user-tooling/"
            ];
          };
          # deadnix.enable = true;
          reuse = {
            enable = true;
            package = pkgs.reuse;
          };
        };
      };
    };
}
