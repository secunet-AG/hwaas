# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ config, ... }:
let
  modules = config.flake.nixosModules;
in
{
  perSystem =
    { pkgs, config, ... }:
    let
      context-api-url-version-prefix = builtins.readFile config.packages.contextapi-version;
      inherit (pkgs.writers) writePython3;
    in
    {
      checks = {
        contextapi-startup-test = pkgs.callPackage ./contextapi-startup-test.nix { inherit modules; };
        openapi-spec-contextapi-golden-test = pkgs.callPackage ./golden-test-openapi-spec.nix {
          inherit (config.packages) contextapi-oas;
        };
        contextapi-images-drives-and-middleware = pkgs.callPackage ./images-drives-and-middleware.nix {
          inherit modules context-api-url-version-prefix;
        };
        contextapi-net-api-test = pkgs.callPackage ./net-api-test.nix {
          inherit modules context-api-url-version-prefix;
        };
        contextapi-remote-hands-aux-device-test = pkgs.callPackage ./remote-hands-aux-device-test.nix {
          inherit writePython3 modules context-api-url-version-prefix;
        };
        contextapi-remote-hands-routing-test = pkgs.callPackage ./remote-hands-routing-test.nix {
          inherit modules context-api-url-version-prefix;
        };
        contextapi-remote-serial-test = pkgs.callPackage ./remote-serial-test.nix {
          inherit modules context-api-url-version-prefix;
        };
        contextapi-test-open-telemetry = pkgs.callPackage ./test-open-telemetry.nix {
          inherit modules context-api-url-version-prefix;
        };
        contextapi-ws-network-routing-test = pkgs.callPackage ./ws-network-routing-test.nix {
          inherit modules context-api-url-version-prefix;
        };
        contextapi-cors-test = pkgs.callPackage ./cors.nix {
          inherit modules context-api-url-version-prefix;
        };
        contextapi-lifetime-test = pkgs.callPackage ./contextapi-lifetime-test.nix {
          inherit modules context-api-url-version-prefix;
        };
      };

      packages = {
        # Same tests as above but enabled for interactive debugging
        contextapi-startup-test-debug = config.checks.contextapi-startup-test.driverInteractive;
        contextapi-images-drives-and-middleware-debug =
          (config.checks.contextapi-images-drives-and-middleware.override { debugging = true; })
          .driverInteractive;
        contextapi-net-api-test-debug = config.checks.contextapi-net-api-test.driverInteractive;
        contextapi-remote-hands-aux-device-test-debug =
          (config.checks.contextapi-remote-hands-aux-device-test.override { debugging = true; })
          .driverInteractive;
        contextapi-remote-hands-routing-test-debug =
          config.checks.contextapi-remote-hands-routing-test.driverInteractive;
        contextapi-remote-serial-test-debug =
          (config.checks.contextapi-remote-serial-test.override { debugging = true; }).driverInteractive;
        contextapi-test-open-telemetry-debug =
          config.checks.contextapi-test-open-telemetry.driverInteractive;
        contextapi-ws-network-routing-test-debug =
          (config.checks.contextapi-ws-network-routing-test.override { debugging = true; }).driverInteractive;
        contextapi-cors-test-debug = config.checks.contextapi-cors-test.driverInteractive;
      };

    };
}
