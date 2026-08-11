# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

# deadnix: skip
perSystem@{ config, ... }:
{ config, lib, ... }:
let
  wsCfg = config.services.websocketProxyGateway;
  username = "wsgateway";
  groupname = "gateway";
  runDir = "/run/ws-gateway";
in
{
  options.services.websocketProxyGateway = {
    enable = lib.mkEnableOption "the Websocket Proxy";

    port = lib.mkOption {
      type = lib.types.port;
      default = 8080;
      example = 80;
      description = "The port used to serve the Websocket Proxy Gateway.";
    };

    address = lib.mkOption {
      type = lib.types.str;
      default = "0.0.0.0";
      example = "127.0.0.1";
      description = "The IP address to serve the Websocket Proxy Gateway.";
    };

    otelEndpoint = lib.mkOption {
      type = lib.types.str;
      default = "http://localhost:4317";
      description = "URL of the OpenTelemetry collector to send traces in otlp format";
    };

    interfacePrefix = lib.mkOption {
      type = lib.types.str;
      default = "wsn";
      description = "Name prefix of the interfaces that will be used by the Websocket Proxy Gateway.
        This prefix will be appended with an ID (e.g. 'vlan' -> vlan1, vlan2, ...)";
    };

    openFirewall = lib.mkEnableOption "opening the firewall for the Websocket Proxy";

  };

  config = lib.mkIf wsCfg.enable {

    users.users."${username}" = {
      isNormalUser = true;
      createHome = false;
      extraGroups = [ "${groupname}" ];
      description = "Websocket Proxy Gateway";
    };

    users.groups."${groupname}" = { };

    systemd.services.websocket-proxy-gateway = {
      description = "HWaaS Websocket L2 Proxy Gateway";
      wantedBy = [ "multi-user.target" ];
      # wait until network is online
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];
      environment = {
        OTEL_SERVICE_NAME = "WsProxyGateway";
        OTEL_EXPORTER_OTLP_ENDPOINT = wsCfg.otelEndpoint;
        OTEL_EXPORTER_OTLP_PROTOCOL = "grpc";
        OTEL_TRACES_SAMPLER = "always_on";
        OTEL_LOG_LEVEL = "info";
      };
      serviceConfig = {
        User = "${username}";
        CapabilityBoundingSet = "CAP_NET_RAW";
        AmbientCapabilities = "CAP_NET_RAW";
        ExecStart = "${perSystem.config.packages.ws-gateway}/bin/ws-gateway -vv ${wsCfg.address}:${builtins.toString wsCfg.port} --dev ${wsCfg.interfacePrefix}";
        WorkingDirectory = "${runDir}";
      };
    };

    # Add run dir for user
    systemd.tmpfiles.rules = [ "d ${runDir} 775 ${username} ${groupname}" ];

    networking.firewall.allowedTCPPorts = lib.optional wsCfg.openFirewall wsCfg.port;
  };
}
