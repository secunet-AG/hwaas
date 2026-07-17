# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ benchmarkDataCollector, pkgs, pre-commit-hooks }:
let
  genericStylecheckExcludes = [
    "nix/sources.nix"
  ];

  hwaasDriver = pkgs.callPackage ../packages/hwaas-driver { inherit benchmarkDataCollector; };
  hwaasTimer = pkgs.python3Packages.callPackage ../packages/hwaas-timer { };

  # To run mypy in a pre-commit check, we have to provide a Python environment
  # that has all used libraries installed.
  mypyPackage = pkgs.python3.withPackages
    (ps: with ps; [
      mypy
      types-requests
      pytest
      responses
      benchmarkDataCollector
      deepmerge
      validators
      opensearch-py
      hwaasDriver
      hwaasTimer
      types-tqdm
    ]);
in
pre-commit-hooks.run {
  src = pkgs.nix-gitignore.gitignoreSource [ ] ../.;

  tools = pkgs;

  hooks = {
    nixpkgs-fmt = {
      enable = true;
      excludes = genericStylecheckExcludes;
    };
    deadnix = {
      enable = true;
      excludes = genericStylecheckExcludes;
    };

    typos = {
      enable = true;
      settings.configPath = ".typos.toml";
    };

    mypy = {
      enable = true;
      settings.binPath = "${mypyPackage}/bin/mypy";
    };

    ruff.enable = true;
    black.enable = true;
  };
}
