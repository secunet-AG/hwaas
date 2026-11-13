# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
# SPDX-License-Identifier: MIT-0

{
  description = "HWaaS test with static local networking for a single BMR";

  inputs = {
    hwaas.url = "github:secunet-AG/hwaas";
    nixpkgs.url = "github:NixOS/nixpkgs";
  };

  outputs = { hwaas, nixpkgs, ... }: {
    lib = hwaas;
    checks.x86_64-linux =
      let
        pkgs = import nixpkgs {
          system = "x86_64-linux";
          config = { };
          overlays = [ ];
        };
        /**
          Import a HWaaS test from a separate file.

          # Inputs

          `test-path`
          : Path to a file containing a single HWaaS test.
        */
        importHwaasTest =
          test-path:
          hwaas.lib.hwaasTest pkgs (
            import test-path {
              inherit (hwaas.nixosModules) user-tooling-hwaasTestVm;
              inherit pkgs;
            }
          );
      in
      {
        my-first-hwaas-test = importHwaasTest ./hwaas-test.nix;
      };
  };
}
