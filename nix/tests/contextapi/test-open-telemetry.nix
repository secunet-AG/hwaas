# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

# This test checks that HTTP requests sent to an application are logged to an OpenTelemetry
# Collector instance.
{
  testers,
  httpie,
  context-api-url-version-prefix,
  modules,
}:
let
  port = 12345;
in
testers.runNixOSTest {
  name = "open-telemetry-test";
  node.specialArgs = { inherit modules; };
  nodes.host = {
    imports = [
      modules.contextapi-module
      modules.test-otel-collector
      ./test-modules/test-config.nix
      ./test-modules/mock-remote-usb.nix
      ./test-modules/mock-contextapi-satellite-rest-services.nix
    ];

    context-api-test-config.enable = true;

    environment.systemPackages = [ httpie ];
    systemd.services.context-api = {
      requires = [ "otel-collector.service" ];
      after = [ "otel-collector.service" ];
    };

    services = {
      # mimic a netctrl instance (enables /power/reset)
      mock-contextapi-satellite-rest-services.enable = true;
      contextApi = {
        enable = true;
        inherit port;
      };
      # mimic a remote-usb instance (enables /usb/reset)
      mock-remote-usb.enable = true;
      otelCollector.enable = true;
    };
  };

  testScript = ''
    start_all()
    host.wait_for_open_port(${builtins.toString port})

    # traceparent header example according to https://www.w3.org/TR/trace-context/#trace-context-http-headers-format
    TRACE_ID = "1a687ed3bd263ba29d721453171b3f3a"
    TRACE_PARENT_HEADER = f"traceparent:00-{TRACE_ID}-004eb04f8b67a8e4-01"

    # make a request that gets traced via OTEL
    responseHeaders = host.succeed(f"http --headers GET :${builtins.toString port}/${context-api-url-version-prefix}/images {TRACE_PARENT_HEADER}")

    # the header must contain the same ID, other parts of the header may change
    assert TRACE_ID in responseHeaders, f"Expected '{TRACE_ID}' to be contained in '{responseHeaders}'"

    # check that the collector received a trace about the submitted request
    host.wait_until_succeeds("journalctl -u otel-collector -g contextapi", timeout = 10)
  '';
}
