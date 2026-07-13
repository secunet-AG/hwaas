# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

# If we are debugging the scenario (e.g. via driverInteractive) there should be
# some extra tools available and graphics support
{ config
, lib
, pkgs
, ...
}:
let
  cfg = config.services.debugging;
in
{
  options.services.debugging = {
    enable = lib.mkEnableOption "the debug module";

    username = lib.mkOption {
      type = lib.types.str;
      default = "nixos";
      description = "The user created for debugging";
    };

    keymap = lib.mkOption {
      type = lib.types.str;
      default = "de";
      example = "en";
      description = "Set the keyboard layout";
    };
  };

  config = lib.mkIf cfg.enable {
    # access the VM directly via qemu screen
    virtualisation.graphics = lib.mkForce true;

    # minify output of 'ip a' by less noise through additional IPv6 adresses
    networking.enableIPv6 = lib.mkDefault false;

    # Add additional utils
    environment.systemPackages = with pkgs; [
      iputils
      tcpdump
      pciutils
      netcat-gnu
      tmux
      jq
      httpie
    ];

    users = {
      mutableUsers = false;
      users = {
        # For ease of debugging the VM as the `root` user
        # If set to an empty string (""), this user will be able to log in without being asked for a password
        root.hashedPassword = "";

        # Create a system user that matches the database user so that we
        # can use peer authentication.  The tutorial defines a password,
        # but it's not necessary.
        "${cfg.username}" = {
          isSystemUser = true;
          group = "${cfg.username}";
        };
      };

      groups.${cfg.username} = { };
    };

    console.keyMap = cfg.keymap;
  };
}
