# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ config, ... }:
let
  # deadnix: skip
  flakeConfig = config;
in
{
  perSystem =
    { pkgs, config, ... }:
    let
      # deadnix: skip
      extendedPkgs = pkgs.appendOverlays [
        (final: _: {
          hwaasTimer = config.packages.user-tooling-hwaasTimer;

          hwaasTest = import ../../user-tooling/packages/hwaas-integration-test {
            pkgs = final;
          };
        })
      ];
    in
    {
      checks = {
        # Current expected issue:
        # error: NameResolutionError on api.hwaas.placeholder.com
        # TODO: use virtualized environment

        # user-tooling-example-simple-pingHWaaSMachine = (import ./simple.nix {
        #   pkgs = extendedPkgs;
        #   hwaasTestModules = flakeConfig.flake.nixosModules;
        # }).pingHWaaSMachine;
        # user-tooling-example-fio-benchmark = import ./fio-benchmark.nix {
        #   pkgs = extendedPkgs;
        #   hwaasTestModules = flakeConfig.flake.nixosModules;
        # };
        # user-tooling-example-boot-time = import ./boot-time.nix {
        #   pkgs = extendedPkgs;
        #   hwaasTestModules = flakeConfig.flake.nixosModules;
        # };
        # user-tooling-example-multiple-machines-singleNetwork = (import ./multiple-machines.nix {
        #   pkgs = extendedPkgs;
        #   hwaasTestModules = flakeConfig.flake.nixosModules;
        # }).singleNetwork;
        # user-tooling-example-multiple-machines-multiNetwork = (import ./multiple-machines.nix {
        #   pkgs = extendedPkgs;
        #   hwaasTestModules = flakeConfig.flake.nixosModules;
        # }).multiNetwork;
        # user-tooling-example-hwaas-connector = (import ./hwaas-connector.nix {
        #   pkgs = extendedPkgs;
        #   hwaasTestModules = flakeConfig.flake.nixosModules;
        # }).hwaasConnector;
        # user-tooling-example-multiple-images-mounting = (import ./multiple-images.nix {
        #   pkgs = extendedPkgs;
        #   hwaasTestModules = flakeConfig.flake.nixosModules;
        # }).mounting;
        # user-tooling-example-multiple-images-bootOrder = (import ./multiple-images.nix {
        #   pkgs = extendedPkgs;
        #   hwaasTestModules = flakeConfig.flake.nixosModules;
        # }).bootOrder;
        # user-tooling-example-request-specific-machine = import ./request-specific-machine.nix {
        #   pkgs = extendedPkgs;
        #   hwaasTestModules = flakeConfig.flake.nixosModules;
        # };
      };
    };
}
