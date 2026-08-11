# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

# deadnix: skip
perSystem@{ config }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  contextCfg = config.services.contextApi;
  username = "contextApi";
  groupname = "gateway";
  runDir = "/run/context-api";

  context-api-config-schema = pkgs.runCommand "generate-context-api-config-schema" { } ''
    ${contextCfg.package}/bin/config-schema-generator --out-file $out
  '';

in
{
  options.services.contextApi = {
    enable = lib.mkEnableOption "the ContextAPI";

    package = lib.mkOption {
      default = perSystem.config.packages.contextapi;
      type = lib.types.package;
      description = ''
        The contextapi package to use. This defaults to the release version.
      '';
    };

    address = lib.mkOption {
      type = lib.types.str;
      default = "0.0.0.0";
      example = "127.0.0.1";
      description = "The IP address to serve the ContextApi under.";
    };

    port = lib.mkOption {
      type = lib.types.port;
      default = 8080;
      example = 80;
      description = "The port used to serve the ContextAPI.";
    };

    config = lib.mkOption {
      type = lib.types.addCheck lib.types.attrs (
        pkgs.callPackage ../lib/inventory-schema-type.nix { configSchema = context-api-config-schema; }

      );
      example = "/user/config.json";
      description = "The path to the file containing the configuration in JSON.";
    };

    otelEndpoint = lib.mkOption {
      type = lib.types.str;
      default = "http://127.0.0.1:4317";
      description = "URL of the OpenTelemetry collector to send traces in otlp format";
    };

    consoleAddress = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      example = "127.0.0.1:6669";
      description = "Start tokio console if a socket address is provided.";
    };

    openFirewall = lib.mkEnableOption "opening the firewall";
  };

  config = lib.mkIf contextCfg.enable {

    users.users."${username}" = {
      isNormalUser = true;
      createHome = false;
      extraGroups = [ "${groupname}" ];
      description = "ContextAPI";
    };

    users.groups."${groupname}" = { };

    systemd.services.context-api =
      let
        tokioConsole = lib.strings.optionalString (
          !builtins.isNull contextCfg.consoleAddress
        ) "--tokio-console-address ${contextCfg.consoleAddress}";
        configFile = pkgs.writeText "contextAPI configuration.json" (builtins.toJSON contextCfg.config);
      in
      {
        description = "HWaaS ContextAPI";
        wantedBy = [ "multi-user.target" ];
        after = [ "network-online.target" ];
        wants = [ "network-online.target" ];
        environment = {
          OTEL_SERVICE_NAME = "ContextAPI";
          OTEL_EXPORTER_OTLP_ENDPOINT = contextCfg.otelEndpoint;
          OTEL_EXPORTER_OTLP_PROTOCOL = "grpc";
          OTEL_TRACES_SAMPLER = "always_on";
          OTEL_LOG_LEVEL = "info";
        };
        serviceConfig = {
          Type = "notify";
          User = "${username}";
          ExecStart = ''
            ${contextCfg.package}/bin/contextapi \
                        -vv \
                        --address ${contextCfg.address} \
                        --port ${builtins.toString contextCfg.port} \
                        --config-file ${configFile} ${tokioConsole}
          '';
          WorkingDirectory = "${runDir}";
          # use custom kill signal to trigger graceful shutdown
          KillSignal = "SIGINT";
          CapabilityBoundingSet = "CAP_NET_RAW";
          AmbientCapabilities = "CAP_NET_RAW";
          TimeoutStartSec = "45min";
        }
        // lib.attrsets.optionalAttrs (contextCfg.port <= 1024) {
          # For ports below 1024 a special capability is needed additionaly (CAP_NET_BIND_SERVICE)
          CapabilityBoundingSet = "CAP_NET_BIND_SERVICE CAP_NET_RAW";
          AmbientCapabilities = "CAP_NET_BIND_SERVICE CAP_NET_RAW";
        };
      };

    systemd.tmpfiles.rules = [ "d ${runDir} 775 ${username} ${groupname}" ];

    networking.firewall.allowedTCPPorts = lib.optional contextCfg.openFirewall contextCfg.port;
  };
}
