# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ config
, lib
, pkgs
, ...
}:
with lib;
let
  cfg = config.services.ws-echo-server;
  echoScript = pkgs.writeShellScript "echo-script.sh" ''
    while read line
    do
      echo "$line"
    done < "''${1:-/dev/stdin}"
  '';
  portString = builtins.toString cfg.port;
in
{
  options.services.ws-echo-server = {
    enable = mkEnableOption "WebSocket echo server";
    port = mkOption {
      type = types.port;
      default = 8080;
    };
  };

  config = mkIf cfg.enable {
    systemd.services.ws-echo-server = {
      description = "REST echo server";
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];
      serviceConfig = {
        ExecStart = "${pkgs.websocketd}/bin/websocketd --port ${portString} ${echoScript}";
        # block until port is actually open
        ExecStartPost = ''
          ${pkgs.coreutils}/bin/timeout 5 ${pkgs.bash}/bin/bash -c '\
            while ! ${pkgs.iproute2}/bin/ss -H -t -l -n sport = :${portString} \
              | ${pkgs.gnugrep}/bin/grep -q "^LISTEN.*:${portString}"; do\
              ${pkgs.coreutils}/bin/sleep 1;\
            done'
        '';
      };
    };
  };
}
