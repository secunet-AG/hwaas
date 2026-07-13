# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

# deadnix: skip
perSystem@{ config, ... }:
{ config
, lib
, pkgs
, ...
}:
let
  cfg = config.services.remote-usb;
  runDir = "/run/remote-usb";
in
{
  options.services.remote-usb = {
    enable = lib.mkEnableOption "Usb Peripheral API";

    package = lib.mkOption {
      default = perSystem.config.packages.remote-usb;
      type = lib.types.package;
      description = ''
        The remote-usb package to use. This defaults to the release version.
      '';
    };

    address = lib.mkOption {
      type = lib.types.str;
      default = "0.0.0.0";
      example = "127.0.0.1";
      description = "The IP address to serve the remote-usb under.";
    };

    port = lib.mkOption {
      type = lib.types.port;
      default = 8080;
      example = 80;
      description = "The port used to serve the remote-usb.";
    };

    configFile = lib.mkOption {
      type = lib.types.str;
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

  config = lib.mkIf cfg.enable {

    systemd.services.remote-usb =
      let
        tokioConsole = lib.strings.optionalString
          (
            !builtins.isNull cfg.consoleAddress
          ) "--tokio-console-address ${cfg.consoleAddress}";
      in
      {
        description = "HWaaS remote-usb";
        wantedBy = [ "multi-user.target" ];
        after = [ "network-online.target" ];
        wants = [ "network-online.target" ];
        environment = {
          OTEL_SERVICE_NAME = "remote-usb";
          OTEL_EXPORTER_OTLP_ENDPOINT = cfg.otelEndpoint;
          OTEL_EXPORTER_OTLP_PROTOCOL = "grpc";
          OTEL_TRACES_SAMPLER = "always_on";
          OTEL_LOG_LEVEL = "info";
        };
        path = with pkgs; [
          bash
          # For modprobe
          "/run/current-system/sw"
        ];
        serviceConfig =
          {
            Type = "notify";
            ExecStart = ''
              ${cfg.package}/bin/remote-usb \
                          -vv \
                          ${tokioConsole} \
                          --address ${cfg.address} \
                          --port ${toString cfg.port} \
                          --config-file ${cfg.configFile}
            '';
            WorkingDirectory = runDir;
            TimeoutStartSec = "45min";
          }
          // lib.optionalAttrs (cfg.port <= 1024) {
            # For ports below 1024 a special capability is needed additionaly (CAP_NET_BIND_SERVICE)
            CapabilityBoundingSet = "CAP_NET_BIND_SERVICE";
            AmbientCapabilities = "CAP_NET_BIND_SERVICE";
          };
      };

    systemd.tmpfiles.rules = [ "d ${runDir} 775 root root" ];

    networking.firewall.allowedTCPPorts = lib.optional cfg.openFirewall cfg.port;
  };
}
