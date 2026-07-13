# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ nixos, isoName }:

# Simple image that opens a serial getty. This is used during the remote-hands USB test.
let
  serialDevice = "ttyS0";
  # TODO: change the device path depending on the used USB OTG serial class
  usbSerialDevice = "ttyACM0";
  myisoconfig =
    { pkgs
    , modulesPath
    , lib
    , ...
    }:
    {
      imports = [
        "${modulesPath}/installer/cd-dvd/installation-cd-minimal.nix"
        "${modulesPath}/profiles/minimal.nix"
      ];

      boot.kernelParams = [
        "nomodeset" # needed for edgebox to boot
      ];

      services = {
        # enable General Purpose Mouse daemon for mouse support in virtual consoles
        gpm.enable = true;
        # udev rule that starts the remote-hands' serial-getty systemd service as soon as the serial device
        # is plugged in
        udev.extraRules = ''
          KERNEL=="${serialDevice}", TAG+="systemd", ENV{SYSTEMD_WANTS}="serial-getty@${serialDevice}"

          KERNEL=="${usbSerialDevice}", TAG+="systemd", ENV{SYSTEMD_WANTS}="serial-getty@${usbSerialDevice}"
        '';
      };

      systemd.services."serial-getty@${serialDevice}" = {
        enable = true;
        serviceConfig = {
          Restart = "always"; # restart when session is closed
          # configure baudrate
          ExecStartPre = ''
            ${pkgs.coreutils}/bin/stty -F /dev/${serialDevice} 115200
          '';
          serialSpeed = [ 115200 ];
        };
      };

      systemd.services."serial-getty@${usbSerialDevice}" = {
        enable = true;
        serviceConfig = {
          Restart = "always"; # restart when session is closed
          # configure baudrate
          ExecStartPre = ''
            ${pkgs.coreutils}/bin/stty -F /dev/${usbSerialDevice} 115200
          '';
          serialSpeed = [ 115200 ];
        };
      };

      documentation = {
        doc.enable = false;
        enable = false;
        info.enable = false;
        man.enable = false;
        nixos.enable = false;
      };

      isoImage.isoName = lib.mkForce isoName;
    };

  myNixos = nixos [ myisoconfig ];
in
myNixos.config.system.build.isoImage
