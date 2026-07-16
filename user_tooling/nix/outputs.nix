# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ pkgs, hwaas ? (import ./nix/sources.nix).hwaas }:
rec {
  nixosModules = import ../modules { inherit pkgs hwaas; };

  packages = {
    hwaasTest = import ../packages/hwaas-integration-test { inherit pkgs; };
    hwaasDataUpload = pkgs.callPackage ../packages/data-upload { };
    hwaasTimer = pkgs.python3Packages.callPackage ../packages/hwaas-timer { };
  };

  overlays.default = _final: _prev: {
    inherit (packages) hwaasTest hwaasTimer;
  };
}
