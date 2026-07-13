# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

# This test checks that HTTP requests sent to an application are logged to an OpenTelemetry
# Collector instance.
{ testers
, httpie
, context-api-url-version-prefix
, modules
,
}:
let
  port = 12345;
in
testers.runNixOSTest {
  name = "cors-test";
  node.specialArgs = { inherit modules; };
  nodes.host = {
    imports = [
      modules.contextapi-module
      ./test-modules/test-config.nix
      ./test-modules/mock-contextapi-satellite-rest-services.nix
      ./test-modules/mock-remote-usb.nix
    ];

    context-api-test-config.enable = true;
    environment.systemPackages = [ httpie ];

    services = {
      # mimic a netctrl instance (enables /power/reset)
      mock-contextapi-satellite-rest-services.enable = true;
      mock-remote-usb.enable = true;
      contextApi = {
        enable = true;
        inherit port;
      };
    };
  };

  testScript = ''
    start_all()
    host.wait_for_open_port(${builtins.toString port})

    # Some origin to mock a request form some web UI
    ORIGIN_HEADER = "Origin:http://ui.example.com"
    METHODE_HEADER = "Access-Control-Request-Method:GET"
    HEADERS_HEADER = "Access-Control-Request-Headers:Content-Type"


    # make a request that gets traced via OTEL
    print(f"Sending CORS preflight request to API server with '{ORIGIN_HEADER}'...\n")
    responseHeaders = host.succeed(f"http --headers GET :${builtins.toString port}/${context-api-url-version-prefix}/images {ORIGIN_HEADER} {METHODE_HEADER} {HEADERS_HEADER}")

    print(responseHeaders)

    # the header must contain the expected CORS headers
    expected_headers = [
      "vary: origin, access-control-request-method, access-control-request-headers",
      "access-control-allow-origin: *"
    ]
    for h in expected_headers:
      assert h in responseHeaders, f"Expected '{h}' to be contained in the response headers"
  '';
}
