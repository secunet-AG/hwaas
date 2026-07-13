# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ lib
, context-api-url-version-prefix
, websocat
, httpie
, testers
, debugging ? false
, modules
,
}:
let
  rsd = import ./rsd.nix;
  ctxPort = "8080";
  tsPort = 9452;
  portHttpEcho = 8765;
  portWs = 8234;
in
testers.runNixOSTest {
  name = "remote-serial-test";
  node.specialArgs = { inherit modules; };
  nodes = {
    sut =
      { config, ... }:
      {
        imports =
          [
            modules.contextapi-module
            ./test-modules/test-config.nix
            ./test-modules/mock-contextapi-satellite-rest-services3.nix
          ]
          ++ lib.optionals debugging [
            ./test-modules/debugging.nix
          ];

        environment.systemPackages = [
          websocat
          httpie
        ];

        context-api-test-config = {
          enable = true;
          remote_usb = "http://localhost:${toString tsPort}/usb";
          remote_serial = "http://localhost:${toString tsPort}/serial";
        };

        services = {
          contextApi = {
            enable = true;
            openFirewall = true;
            port = lib.toInt ctxPort;
          };
          reverse-proxy = {
            enable = true;
            port = tsPort;
            port-http-echo = portHttpEcho;
            port-ws = portWs;
          };

        };

      };
  };

  testScript = ''
    import json
    start_all()
    sut.wait_for_open_port(${builtins.toString portHttpEcho})
    sut.wait_for_open_port(${builtins.toString portWs})
    sut.wait_for_open_port(${builtins.toString tsPort})
    sut.wait_for_open_port(${builtins.toString ctxPort})
    sut.wait_for_unit("context-api.service")

    base_url = "127.0.0.1:${ctxPort}/${context-api-url-version-prefix}/contexts"
    rsd_object_string = json.dumps(${rsd})
    context_uuid = sut.succeed(f"http --check-status POST http://{base_url} <<<'{rsd_object_string}'")
    # Get a list of all serial devices
    getSerial = sut.succeed(f"http --check-status GET http://{base_url}/{context_uuid}/machines/abmr/serial")
    getSerialJson = json.loads(getSerial)
    # We have hard coded the mock server to just return a single serial device named "0"
    assert getSerialJson[0] == "0"
    # Setup a websocket connection to the serial device
    expected = "foo"
    # use --no-close to wait for one incoming message
    result = sut.succeed(f"websocat --one-message --no-close ws://{base_url}/{context_uuid}/machines/abmr/serial/0/websocket <<< {expected}", timeout=30).strip()
    assert result == expected, f"Expected {expected} but received {result}"

    # Check that we also can communicate with the serial device using PUT
    putResult = sut.succeed(f"http --check-status PUT http://{base_url}/{context_uuid}/machines/abmr/serial/0 <<< {expected}")
    putResultJson = json.loads(putResult)
    assert putResultJson["path"] == "/serial/0"

    # Check that we can clear the buffer for the serial device using DELETE
    sut.succeed(f"http --check-status DELETE http://{base_url}/{context_uuid}/machines/abmr/serial/0")

    # And we should also be able to clear all the machine's serial devices with POST /serial/reset
    resetResponse = sut.succeed(f"http --check-status POST http://{base_url}/{context_uuid}/machines/abmr/serial/reset")
    resetResponseJson = json.loads(resetResponse)
    print(resetResponseJson)
    assert resetResponseJson["path"] == "/serial/reset"
  '';
}
