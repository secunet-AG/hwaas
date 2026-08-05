# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

# deadnix: skip
perSystem@{ config, ... }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  netCtrlCfg = config.services.netCtrl;
  username = "netctrl";
  groupname = "gateway";
  runDir = "/run/netctrl";
  logFilePath = "${runDir}/debug.log";

  net-ctrl-inventory-schema = pkgs.runCommand "generate-net-ctrl-inventory-openapi-json" { } ''
    ${netCtrlCfg.package}/bin/net-ctrl-config-schema-generator --out-file $out
  '';

in
{
  options.services.netCtrl = {
    enable = lib.mkEnableOption "the Network Controller";

    package = lib.mkOption {
      default = perSystem.config.packages.net-ctrl;
      type = lib.types.package;
      description = ''
        The net-ctrl package to use. This defaults to the release version.
      '';
    };

    port = lib.mkOption {
      type = lib.types.port;
      default = 8000;
      example = 80;
      description = "The port used to serve the NetCtrl REST API. For Ports below 1025 the capability CAP_NET_BIND_SERVICE is granted.";
    };

    address = lib.mkOption {
      type = lib.types.str;
      default = "0.0.0.0";
      example = "127.0.0.1";
      description = "The IP address to serve the NetCtrl REST API.";
    };

    enableLogFile = lib.mkOption {
      type = lib.types.bool;
      default = true;
      example = false;
      description = "If true, a tracing log formated as JSON is appended to '${logFilePath}'";
    };

    inventory = lib.mkOption {
      type = lib.types.addCheck lib.types.attrs (
        pkgs.callPackage ../lib/inventory-schema-type.nix { configSchema = net-ctrl-inventory-schema; }
      );
      default = { };
      example = { };
      description = "Information about available switches.";
    };

    otelEndpoint = lib.mkOption {
      type = lib.types.str;
      default = "http://localhost:4317";
      description = "URL of the OpenTelemetry collector to send traces in otlp format";
    };

    openFirewall = lib.mkEnableOption "opening the firewall for the NetCtrl";

  };

  config = lib.mkIf netCtrlCfg.enable {

    users.users."${username}" = {
      isNormalUser = true;
      createHome = false;
      extraGroups = [ "${groupname}" ];
      description = "NetCtrl";
    };

    users.groups."${groupname}" = { };

    systemd.services.net-ctrl =
      let
        netCtrlInventory = pkgs.writeText "netCtrl inventory.json" (builtins.toJSON netCtrlCfg.inventory);
        logFileOpt = lib.optionalString netCtrlCfg.enableLogFile "--log-file ${logFilePath}";
        socketAddr = "${netCtrlCfg.address}:${builtins.toString netCtrlCfg.port}";
      in
      {
        description = "HWaaS Network Controller";
        wantedBy = [ "multi-user.target" ];
        # wait until network is online
        after = [ "network-online.target" ];
        wants = [ "network-online.target" ];
        environment = {
          OTEL_SERVICE_NAME = "NetCtrl";
          OTEL_EXPORTER_OTLP_ENDPOINT = netCtrlCfg.otelEndpoint;
          OTEL_EXPORTER_OTLP_PROTOCOL = "grpc";
          OTEL_TRACES_SAMPLER = "always_on";
          OTEL_LOG_LEVEL = "info";
        };
        serviceConfig = {
          User = "${username}";
          ExecStart = "${netCtrlCfg.package}/bin/net-ctrl -vv ${logFileOpt} --inventory-file ${netCtrlInventory} ${socketAddr}";
          WorkingDirectory = runDir;
        }
        // lib.attrsets.optionalAttrs (netCtrlCfg.port <= 1024) {
          # For ports below 1024 a special capability is needed (CAP_NET_BIND_SERVICE)
          CapabilityBoundingSet = "CAP_NET_BIND_SERVICE";
          AmbientCapabilities = "CAP_NET_BIND_SERVICE";
        };
      };

    # Add run dir for user
    systemd.tmpfiles.rules = [ "d ${runDir} 775 ${username} ${groupname}" ];

    networking.firewall.allowedTCPPorts = lib.optional netCtrlCfg.openFirewall netCtrlCfg.port;
  };
}
