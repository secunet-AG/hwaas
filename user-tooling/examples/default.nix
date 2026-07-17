# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ pkgs
, hwaas
}:
let
  # None of these examples will run, since the URL is a placeholder
  # and the machines do not exist
  hwaasTest = import ../nix/outputs.nix { inherit pkgs hwaas; };

  extendedPkgs = pkgs.appendOverlays [
    hwaasTest.overlays.default
  ];
in
{
  simple = import ./simple.nix {
    pkgs = extendedPkgs;
    hwaasTestModules = hwaasTest.nixosModules;
  };
  fio-benchmark = import ./fio-benchmark.nix {
    pkgs = extendedPkgs;
    hwaasTestModules = hwaasTest.nixosModules;
  };
  boot-time = import ./boot-time.nix {
    pkgs = extendedPkgs;
    hwaasTestModules = hwaasTest.nixosModules;
  };
  multiple-machines = import ./multiple-machines.nix {
    pkgs = extendedPkgs;
    hwaasTestModules = hwaasTest.nixosModules;
  };
  hwaas-connector = import ./hwaas-connector.nix {
    pkgs = extendedPkgs;
    hwaasTestModules = hwaasTest.nixosModules;
  };
  multiple-images = import ./multiple-images.nix {
    pkgs = extendedPkgs;
    hwaasTestModules = hwaasTest.nixosModules;
  };
  request-specific-machine = import ./request-specific-machine.nix {
    pkgs = extendedPkgs;
    hwaasTestModules = hwaasTest.nixosModules;
  };
}
