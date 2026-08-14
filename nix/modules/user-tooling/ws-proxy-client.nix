# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

ws-proxy:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  wsCfg = config.services.websocketProxyClient;
  username = "wsclient";

  serviceCapabilities = [ "CAP_NET_ADMIN" ];

  networkOptions = lib.types.submodule {
    options = {
      uri = lib.mkOption {
        type = with lib.types; nullOr str;
        default = null;
        description = "The websocket endpoint URI to connect.";
      };

      envFile = lib.mkOption {
        type = with lib.types; nullOr path;
        default = null;
        description = "Path to a file that contains the URI as environment setting WS_PROXY_URI.";
      };
    };
  };

  mkServices = lib.mapAttrs' (
    iface: net:
    lib.nameValuePair "websocket-proxy-client-${iface}" {
      description = "HWaaS Websocket L2 Proxy Client for ${iface}";
      wantedBy = [ "multi-user.target" ];
      # wait until network is online
      requires = [ "network-online.target" ];
      after = [ "network-online.target" ];
      serviceConfig = {
        Type = "notify";
        User = "${username}";
        Group = "${config.users.users."${username}".group}";
        Restart = "on-failure";
        RestartSec = 5;
        CapabilityBoundingSet = serviceCapabilities;
        AmbientCapabilities = serviceCapabilities;
        ExecStart = "${pkgs.bash}/bin/bash -c '${ws-proxy}/bin/ws-proxy-client -vv --address $WS_PROXY_URI ${iface}'";
        Environment = lib.optionalString (net.uri != null) "WS_PROXY_URI=${net.uri}";
        EnvironmentFile = lib.optionalString (net.envFile != null) net.envFile;
      };
      unitConfig = {
        StartLimitBurst = 5;
        StartLimitIntervalSec = 30;
      };
    }
  );
in
{
  options.services.websocketProxyClient = {
    enable = lib.mkEnableOption ''
      The Websocket Proxy Client

      The Websocket Proxy Client module is used to establish a VLAN connection to a HWaaS network via WebSockets.
      It creates a virtual Tap network device where the Traffic to a HWaaS network could be routed to allow
      communication between the HWaaS context machines and the device where this service is activated.
    '';

    networks = lib.mkOption {
      type = lib.types.attrsOf networkOptions;
      default = { };
      example = {
        tap0 = {
          uri = "ws://192.168.1.24/ws/1";
        };
        tap1 = {
          envFile = "/var/lib/hwaas/network.conf";
        };
      };
      description = "List of websocket interfaces/networks to connect.";
    };
  };

  config = lib.mkIf wsCfg.enable {

    assertions = builtins.attrValues (
      builtins.mapAttrs (_: net: {
        assertion = (net.uri != null) != (net.envFile != null);
        message = ''
          Either uri or envFile is required for each websocketProxyClient network entry.
                            Please specify exactly one.'';
      }) wsCfg.networks
    );

    users.users.${username} = {
      isNormalUser = true;
      createHome = false;
      description = "Websocket Proxy Client";
    };

    systemd.services = mkServices wsCfg.networks;
  };
}
