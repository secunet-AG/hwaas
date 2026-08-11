# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ config, ... }:
let
  modules = config.flake.nixosModules;
in
{
  perSystem = { pkgs, ... }: {
    checks = {
      aruba-switch-mock-integration-test = pkgs.callPackage ./integration-test.nix { inherit modules; };
      aruba-switch-mock-login-test = pkgs.callPackage ./login-test.nix { inherit modules; };
      aruba-switch-mock-stats-test = pkgs.callPackage ./stats-test.nix { inherit modules; };
      aruba-switch-mock-ports-test = pkgs.callPackage ./ports-test.nix { inherit modules; };
    };
  };
}
