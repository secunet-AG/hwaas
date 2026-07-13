# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

# deadnix: skip
perSystem@{ config }:
{ pkgs
, config
, lib
, ...
}:
let
  arubaDummyCfg = config.services.arubaDummySwitch;
in
{
  options.services.arubaDummySwitch = {
    enable = lib.mkEnableOption "the dummy switch";

    port = lib.mkOption {
      type = lib.types.port;
      default = 8045;
      example = 80;
      description = "The port used to serve the dummy switch";
    };

    address = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      example = "0.0.0.0";
      description = "The IP address to serve the dummy switch";
    };

    openFirewall = lib.mkEnableOption "opening the firewall for the NetCtrl";
  };

  config = lib.mkIf arubaDummyCfg.enable {
    systemd.services.aruba-dummy-switch =
      let
        server-address = "${arubaDummyCfg.address}:${builtins.toString arubaDummyCfg.port}";
      in
      {
        description = "Aruba dummy switch";
        wantedBy = [ "multi-user.target" ];
        after = [ "network-online.target" ];
        wants = [ "network-online.target" ];
        serviceConfig = {
          #WorkingDirectory = "${restapiEchoServer}/src";
          ExecStart = "${perSystem.config.packages.aruba-switch-mock}/bin/aruba-switch-mock -vv ${server-address}";
          # block until port is actually open
          ExecStartPost = ''
            ${pkgs.coreutils}/bin/timeout 5 ${pkgs.bash}/bin/bash -c '\
              while ! ${pkgs.iproute2}/bin/ss -H -t -l -n sport = :${builtins.toString arubaDummyCfg.port} \
                | ${pkgs.gnugrep}/bin/grep -q "^LISTEN.*:${builtins.toString arubaDummyCfg.port}"; do\
                ${pkgs.coreutils}/bin/sleep 1;\
              done'
          '';
        };
      };
  };
}
