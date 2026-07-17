# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ system ? builtins.currentSystem
, sources ? import ./nix/sources.nix
, pkgs ? import sources.nixpkgs { inherit system; }
, pre-commit-hooks ? import sources."pre-commit-hooks.nix"
}:
let
  hwaas = import sources.hwaas;
  outputs = import ./nix/outputs.nix { inherit pkgs hwaas; };
in
rec {
  checks = import ./tests {
    inherit pkgs pre-commit-hooks;
    inherit (packages) benchmarkDataCollector;
  };

  examples = import ./examples { inherit pkgs hwaas; };

  packages = outputs.packages // rec {
    benchmarkDataCollector = pkgs.callPackage ./packages/benchmark-data-collector { };
    hwaasPythonDriver = pkgs.callPackage ./packages/hwaas-driver { inherit benchmarkDataCollector; };
  };

  inherit (outputs) nixosModules;
}
