# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ ... }: {
  system.stateVersion = "23.11";
  nixpkgs.hostPlatform = "x86_64-linux";

  # These kernel modules are required to
  # let the kernel detect the USB drives
  # after the kernel has booted from the USB disk
  boot.initrd.availableKernelModules = [
    "xhci_pci"
    "usb_storage"
  ];

  # The default user which is logged in automatically after the machine has started
  users.users.nixos = {
    isNormalUser = true;
  };

  services.getty.autologinUser = "nixos";

  # Allow ssh access on the system under test
  services.openssh.enable = true;

  # Allow console via the HWaaS machine serial API
  console.enable = true;
  boot.kernelParams = [
    "console=tty0"
    "console=ttyS0,115200n8"
  ];
}
