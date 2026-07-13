# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

# This test checks that HTTP requests sent to an application are logged to an OpenTelemetry
# Collector instance.
{ nixosTest
, httpie
, modules
,
}:
let
  port = 12345;
in
nixosTest {
  name = "open-telemetry-test";

  nodes.host = {
    imports = [
      modules.ws-gateway-module
      modules.test-otel-collector
    ];

    environment.systemPackages = [ httpie ];

    systemd.services.websocket-proxy-gateway.requires = [ "otel-collector.service" ];
    services = {
      websocketProxyGateway = {
        enable = true;
        inherit port;
      };
      otelCollector.enable = true;
    };
  };

  testScript = ''
    start_all()
    host.wait_for_open_port(${builtins.toString port})
    host.wait_for_unit("otel-collector.service")

    # traceparent header example according to https://www.w3.org/TR/trace-context/#trace-context-http-headers-format
    TRACE_ID = "1a687ed3bd263ba29d721453171b3f3a"
    TRACE_PARENT_HEADER = f"traceparent:00-{TRACE_ID}-004eb04f8b67a8e4-01"

    # make a request that gets traced via OTEL
    responseHeaders = host.succeed(f"http --headers GET :${builtins.toString port}/ws/123 {TRACE_PARENT_HEADER}")

    # the header must contain the same ID, other parts of the header may change
    assert TRACE_ID in responseHeaders, f"Expected '{TRACE_ID}' to be contained in '{responseHeaders}'"

    # check that the collector received a trace about the submitted request
    host.wait_until_succeeds("journalctl -u otel-collector -g WsProxyGateway", timeout = 10)
  '';
}
