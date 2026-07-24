# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ inputs, ... }:
let
  # To run mypy in a pre-commit check, we have to provide a Python environment
  # that has all used libraries installed.
  mypyPackage = pkgs: config: pkgs.python3.withPackages
    (ps: with ps; [
      mypy
      types-requests
      pytest
      responses
      config.packages.user-tooling-benchmarkDataCollector
      deepmerge
      validators
      opensearch-py
      config.packages.user-tooling-hwaasPythonDriver
      config.packages.user-tooling-hwaasTimer
      types-tqdm
    ]);
in
{
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
          # enable user-tooling pre-commit checks as well
          # TODO: run these on the full repository
          typos = {
            enable = true;
            settings.configPath = "./user-tooling/.typos.toml";
            files = "^user-tooling/";
          };
          mypy = {
            enable = true;
            # Force the hook to run as one process rather than parallel hook batches to avoid locks.
            require_serial = true;
            settings.binPath = "${mypyPackage pkgs config}/bin/mypy";
            files = "^user-tooling/.*\\.py$";
          };
          ruff = {
            enable = true;
            files = "^user-tooling/.*\\.py$";
          };
          black = {
            enable = true;
            files = "^user-tooling/.*\\.py$";
          };
        };
      };
    };
}
