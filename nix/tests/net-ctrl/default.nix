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
    {
      checks = {
        net-ctrl-get-switches = pkgs.callPackage ./get-switches.nix {
          inherit modules;
        };
        net-ctrl-golden-test-openapi-spec = pkgs.callPackage ./golden-test-openapi-spec.nix {
          oas = config.packages.net-ctrl-oas;
        };
        net-ctrl-test-open-telemetry = pkgs.callPackage ./test-open-telemetry.nix {
          inherit modules;
        };
        net-ctrl-test-switch-client-cache = pkgs.callPackage ./test-switch-client-cache.nix {
          inherit modules;
        };
      };
    };
}
