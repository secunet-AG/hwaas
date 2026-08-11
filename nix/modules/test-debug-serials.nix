# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{
  lib,
  pkgs,
  config,
  ...
}:
let
  cfg = config.services.debug-serials;
in
{
  options.services.debug-serials = {
    enable = lib.mkEnableOption "debug serials";
    earlyprintk = lib.mkEnableOption "earlyprintk to get kernel messages";
    serials = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [
        "ttyS0"
        "ttyUSB0"
      ];
    };
    earlyConsole = lib.mkEnableOption "add kernel comand line to use the serials early";
  };

  config = lib.mkIf cfg.enable {
    systemd.services = builtins.listToAttrs (
      map (dev: {
        name = lib.strings.concatStrings [
          "serial-getty@"
          dev
        ];
        value = {
          enable = true;
          serviceConfig = {
            Restart = "always";

            # configure baudrate
            # the serial getty module looks like it would set the baud rate,
            # but it doesn't.
            ExecStartPre = ''
              ${pkgs.coreutils}/bin/stty -F /dev/${dev} 115200 raw
            '';
            serialSpeed = [ 115200 ];
          };
        };
      }) cfg.serials
    );

    boot.kernelParams =
      lib.optionals cfg.earlyConsole (map (c: "console=${c},115200n8") cfg.serials)
      ++ lib.optionals cfg.earlyprintk (map (c: "earlyprintk=${c}") cfg.serials);
    boot.loader.grub.extraConfig = lib.optionalString cfg.earlyConsole ''
      serial --speed=115200 --unit=0 --word=8 --parity=no --stop=1
      terminal_input serial
      terminal_output serial
    '';

    # launch as soon as udev sees the device pop up
    services.udev.extraRules = lib.strings.concatMapStrings (tty: ''
      KERNEL=="${tty}", TAG+="systemd", ENV{SYSTEMD_WANTS}="serial-getty@${tty}"
    '') cfg.serials;
  };
}
