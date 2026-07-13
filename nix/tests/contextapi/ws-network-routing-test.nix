# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ testers
, lib
, context-api-url-version-prefix
, debugging ? false
, modules
,
}:
let
  serverPort = 8080;
  netName = "my-net";
  wsEchoPort = 8234;
  ws_echo_url = "ws://127.0.0.1:${builtins.toString wsEchoPort}";
  rsd = import ./rsd.nix;
in
testers.runNixOSTest {
  name = "ws-network-routing-test";
  node.specialArgs = { inherit modules; };
  nodes.gateway =
    { config, pkgs, ... }:
    {
      imports =
        [
          modules.contextapi-module
          ./test-modules/test-config.nix
          ./test-modules/mock-contextapi-satellite-rest-services.nix
          ./test-modules/mock-remote-usb.nix
          ./test-modules/ws-gateway-mock.nix
        ]
        ++ lib.optionals debugging [
          ./test-modules/debugging.nix
        ];

      context-api-test-config.enable = true;

      services = {
        mock-contextapi-satellite-rest-services.enable = true;
        mock-remote-usb.enable = true;
        ws-gateway-mock = {
          enable = true;
          port = wsEchoPort;
        };

        contextApi = {
          enable = true;
          openFirewall = true;
          port = serverPort;
        };
      };
    };

  testScript = ''
    import json
    start_all()
    gateway.wait_for_unit("websocat-server.service")
    gateway.wait_for_unit("context-api.service")
    gateway.wait_for_open_port(${builtins.toString serverPort})


    # Test if we can reach websocat directly
    gateway.succeed("websocat ${ws_echo_url} <<< \"Hallo\"")
    gateway.succeed("journalctl -u websocat-server.service --grep 'Hallo'")

    base_url = "127.0.0.1:${builtins.toString serverPort}/${context-api-url-version-prefix}/contexts"

    rsd_object_string = json.dumps(${rsd})
    context_uuid = gateway.succeed(f"curl --fail --silent -X POST -H 'Content-Type: application/json' --data '{rsd_object_string}' {base_url}")

    context_url = f"{base_url}/{context_uuid}"
    # To access a network via the ContextAPI it must be allocated before.
    # So putting an interface of our "fake" SUT into a new network will allocate it.

    body = "{\"abmr\": {\"lan1\": {}}}"
    gateway.succeed(f"curl --fail --silent -X PUT -H 'Content-Type: application/json' --data '{body}' {context_url}/networks/${netName}" )

    # Now the network should exist within the ContextAPI
    # and the websocket connection could be established
    # Ping the SUT via the TAP with the websocket backend
    gateway.succeed(f"websocat ws://{context_url}/networks/${netName}/websocket <<< \"Foo\"")
    gateway.succeed("journalctl -u websocat-server.service --grep 'Foo'")

    # ... retry one time
    gateway.succeed(f"websocat ws://{context_url}/networks/${netName}/websocket <<< \"Bar\"")
    gateway.succeed("journalctl -u websocat-server.service --grep 'Bar'")

    # We know the only valid NetworkID is '3'. There should now be a expected ammount of
    # connection attempts within our websocat-server log.
    res = gateway.succeed("journalctl -u websocat-server.service --grep '/ws/3' | wc -l").strip()
    assert str(res) == "2", "unexpected amount of connection attempts"

  '';
}
