# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

# OpenTelemetry collector module for use in NixOS integration tests
{
  config,
  lib,
  pkgs,
  ...
}:
let
  otelCfg = config.services.otelCollector;

  # Config of an otel collector that collects via http and prints all collected traces to stdout
  otelConfig = pkgs.writeText "otel-collector-config" ''
    receivers:
      otlp:
        protocols:
          grpc:

    exporters:
      debug:
        verbosity: detailed


    service:
      pipelines:
        traces:
          receivers: [otlp]
          exporters: [debug]
        metrics:
          receivers: [otlp]
          exporters: [debug]
        logs:
          receivers: [otlp]
          exporters: [debug]
  '';
in
{
  options.services.otelCollector = {
    enable = lib.mkEnableOption "the OpenTelemetry collector";
  };

  config = lib.mkIf otelCfg.enable {
    systemd.services.otel-collector = {
      description = "OpenTelemetry Collector";
      serviceConfig.ExecStart = "${pkgs.opentelemetry-collector}/bin/otelcol --config=file:${otelConfig}";
    };
  };
}
