# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ config, ... }:
let
  modules = config.flake.nixosModules;
in
{
  perSystem = { pkgs, config, ... }: {
    checks = {
      user-tooling-checkTestconfig = pkgs.callPackage ./check-testconfig.nix { };
      user-tooling-image-compression = pkgs.callPackage ./image-compression.nix {
        inherit modules;
        context-api-url-version-prefix = builtins.readFile config.packages.contextapi-version;
        hwaasPythonDriver = config.packages.user-tooling-hwaasPythonDriver;
      };
    };

    packages = {
      user-tooling-image-compression-debug =
        (config.checks.user-tooling-image-compression.override { debugging = true; }).driverInteractive;
    };
  };
}
