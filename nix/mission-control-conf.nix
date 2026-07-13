# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ lib, ... }:
{
  perSystem =
    { pkgs, config, ... }:
    {
      mission-control = {
        banner = ''
          echo "Welcome to the HWaaS Project" && ,
        '';
        scripts = {
          hello = {
            description = "Say Hello";
            exec = "echo Hello";
          };
          fmt = {
            description = "Format the top-level Nix files";
            exec = "${lib.getExe pkgs.nixfmt} ./*.nix";
            category = "Tools";
          };
          copy-net-ctrl-client = {
            description = "Copy net-ctrl-client to contextapi";
            exec = "${pkgs.rsync}/bin/rsync -r --chmod=+w ${config.packages.net-ctrl-client}/ components/contextapi/net_ctrl_client";
          };
        };
      };
    };
}
