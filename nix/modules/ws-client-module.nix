# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

# deadnix: skip
perSystem@{ config, ... }:
{ config, lib, ... }:
let
  wsCfg = config.services.websocketProxyClient;
  username = "wsclient";
in
{
  options.services.websocketProxyClient = {
    enable = lib.mkEnableOption "the Websocket Proxy Client";

    uri = lib.mkOption {
      type = lib.types.str;
      example = "ws://192.168.1.24/ws/1";
      description = "The URI for the websocket to connect.";
    };

    baseInterface = lib.mkOption {
      type = lib.types.str;
      example = "tap0";
      description = "The linux network interface to L2 Packets via AF_PACKETS";
    };

  };

  config = lib.mkIf wsCfg.enable {

    users.users.${username} = {
      isNormalUser = true;
      createHome = false;
      description = "Websocket Proxy Client";
    };

    systemd.services.websocket-proxy-client = {
      description = "HWaaS Websocket L2 Proxy Client";
      wantedBy = [ "multi-user.target" ];
      # wait until network is online
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];
      serviceConfig = {
        Type = "notify";
        User = "${username}";
        Restart = "on-failure";
        RestartSec = 5;
        CapabilityBoundingSet = "CAP_NET_ADMIN";
        AmbientCapabilities = "CAP_NET_ADMIN";
        ExecStart = "${perSystem.config.packages.ws-proxy-client}/bin/ws-proxy-client -vv --address ${wsCfg.uri} ${wsCfg.baseInterface}";
      };
      unitConfig = {
        StartLimitBurst = 5;
        StartLimitIntervalSec = 30;
      };
    };
  };
}
