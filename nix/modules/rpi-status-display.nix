# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

# deadnix: skip
perSystem@{ config, ... }:
{ config, lib, ... }:
let
  rpiStatusDisplayCfg = config.services.rpi-status-display;
  username = "status-display";
  groupname = "gateway";
  runDir = "/run/rpi-status-display";
in
{
  options.services.rpi-status-display = {
    enable = lib.mkEnableOption "the RPI status display";

    text = lib.mkOption {
      type = lib.types.str;
      example = "Hello World";
      description = "Text to print on the display";
    };
  };

  config = lib.mkIf rpiStatusDisplayCfg.enable {

    users.users."${username}" = {
      isNormalUser = true;
      createHome = false;
      extraGroups = [
        "${groupname}"
        "i2c"
      ];
      description = "RPI status display";
    };

    users.groups."${groupname}" = { };

    systemd.services.rpi-status-display = {
      description = "HWaaS RPI status display";
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        User = "${username}";
        ExecStart = "${perSystem.config.packages.rpi-status-display}/bin/rpi-status-display -vv --text '${rpiStatusDisplayCfg.text}'";
        WorkingDirectory = runDir;
      };
    };

    # Add run dir for user
    systemd.tmpfiles.rules = [ "d ${runDir} 775 ${username} ${groupname}" ];
  };
}
