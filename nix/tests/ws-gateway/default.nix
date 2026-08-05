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
      ws-gateway-test-connect = pkgs.callPackage ./test-connect.nix {
        inherit (pkgs.testers) nixosTest;
        inherit modules;

        inherit (config.packages) ws-proxy-client;
      };
      ws-gateway-test-open-telemetry = pkgs.callPackage ./test-open-telemetry.nix {
        inherit (pkgs.testers) nixosTest;
        inherit modules;

      };
      ws-gateway-test-clients-and-suts-scenario = pkgs.callPackage ./test-clients-and-suts-scenario.nix {
        inherit (pkgs.testers) nixosTest;
        inherit modules;

      };
      ws-gateway-test-many-clients-scenario = pkgs.callPackage ./test-many-clients-scenario.nix {
        inherit (pkgs.testers) nixosTest;
        inherit modules;

      };
      ws-gateway-test-many-vlans-ping = pkgs.callPackage ./test-many-vlans-ping.nix {
        inherit (pkgs.testers) nixosTest;
        inherit modules;
      };
    };
    packages = {
      # Same tests as above but enabled for interactive debugging
      ws-gateway-test-connect-debug =
        (config.checks.ws-gateway-test-connect.override { debugging = true; }).driverInteractive;
      ws-gateway-test-many-vlans-ping-debug =
        (config.checks.ws-gateway-test-many-vlans-ping.override { debugging = true; }).driverInteractive;
      ws-gateway-test-many-clients-scenario-debug =
        (config.checks.ws-gateway-test-many-clients-scenario.override { debugging = true; })
        .driverInteractive;
      ws-gateway-test-clients-and-suts-scenario-debug =
        (config.checks.ws-gateway-test-clients-and-suts-scenario.override { debugging = true; })
        .driverInteractive;
    };
  };
}
