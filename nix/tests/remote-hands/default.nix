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
      checks =
        {
          remote-hands-check-otel-traces = pkgs.callPackage ./check-otel-traces.nix {
            inherit modules;
          };
          remote-hands-golden-test-openapi-spec = pkgs.callPackage ./golden-test-openapi-spec.nix {
            inherit (config.packages) remote-hands-oas;
          };
          remote-hands-remote-auxiliary = pkgs.callPackage ./remote-auxiliary.nix {
            inherit modules;
          };
          remote-hands-remote-power = pkgs.callPackage ./remote-power.nix {
            inherit modules;
          };
          remote-hands-remote-serial-echo = pkgs.callPackage ./remote-serial-echo.nix {
            inherit modules;
          };

          # TODO: This test needs to run on actual Hardware as it seams
          remote-hands-remote-usb = pkgs.callPackage ./remote-usb.nix {
            image =
              let
                isoName = "usb-test-image.iso";
                imageDrv = import ./usb_test_image.nix {
                  inherit (pkgs) nixos;
                  inherit isoName;
                };
              in
              "${imageDrv}/iso/${isoName}";
            utils = pkgs.writeShellScriptBin "raspi_utils.sh" (builtins.readFile ./raspi_utils.sh);
          };
        }
        // (builtins.removeAttrs (pkgs.callPackage ./remote-serial-udev.nix { inherit modules; }) [
          "override"
          "overrideDerivation"
        ]);
    };
}
