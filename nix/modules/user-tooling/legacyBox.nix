# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ hwaasTestLegacyBios }: { pkgs, lib, ... }: {
  imports = [
    # Nixos installer image with legacy bios support
    hwaasTestLegacyBios
  ];

  #########################
  # Network Configuration #
  #########################
  networking = {
    firewall.enable = false;
    useDHCP = pkgs.lib.mkForce true;
    networkmanager.enable = true;
    wireless.enable = lib.mkForce false;
    enableIPv6 = false;
  };

  ################################################
  # Network Device assignment to interface names #
  ################################################
  services.udev.extraRules = ''
    SUBSYSTEM=="net", ACTION=="add", DRIVERS=="igb", KERNELS=="0000:02:00.0", NAME="lan4"
    SUBSYSTEM=="net", ACTION=="add", DRIVERS=="igb", KERNELS=="0000:03:00.0", NAME="lan3"
    SUBSYSTEM=="net", ACTION=="add", DRIVERS=="igb", KERNELS=="0000:04:00.0", NAME="lan2"
    SUBSYSTEM=="net", ACTION=="add", DRIVERS=="igb", KERNELS=="0000:05:00.0", NAME="lan1"
    SUBSYSTEM=="net", ACTION=="add", DRIVERS=="igb", KERNELS=="0000:09:00.0", NAME="sfp1"
    SUBSYSTEM=="net", ACTION=="add", DRIVERS=="igb", KERNELS=="0000:09:00.1", NAME="sfp2"
  '';
}
