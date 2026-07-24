# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ inputs, ... }:
let
  mypyPackage = pkgs: config: pkgs.python3.withPackages (ps: with ps; [
    mypy types-requests pytest responses
    config.packages.user-tooling-benchmarkDataCollector
    deepmerge validators opensearch-py
    config.packages.user-tooling-hwaasPythonDriver
    config.packages.user-tooling-hwaasTimer
    types-tqdm
  ]);
in
{
  imports = [ inputs.git-hooks-nix.flakeModule ];
  perSystem = { config, pkgs, ... }: {
    pre-commit = {
      check.enable = true;
      inherit pkgs;
      settings.hooks = {
        treefmt = {
          enable = true;
          packageOverrides.treefmt = config.treefmt.build.wrapper;
        };
        statix.enable = true;
        deadnix.enable = true;
        reuse = { enable = true; package = pkgs.reuse; };
        # User Tooling Specific
        typos = {
          enable = true;
          settings.configPath = "./user-tooling/.typos.toml";
          files = "^user-tooling/";
        };
        mypy = {
          enable = true;
          require_serial = true;
          settings.binPath = "${mypyPackage pkgs config}/bin/mypy";
          files = "^user-tooling/.*\\.py$";
        };
        ruff = { enable = true; files = "^user-tooling/.*\\.py$"; };
        black = { enable = true; files = "^user-tooling/.*\\.py$"; };
      };
    };
  };
}