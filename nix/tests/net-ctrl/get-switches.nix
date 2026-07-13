# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ testers
, debugging ? false
, curl
, modules
,
}:
let
  netCtrlPort = 80;
  inventory = import ./inventory.nix;
in
testers.nixosTest {
  name = "net-ctrl-get-switches";
  nodes = {
    server = {
      imports = [
        modules.test-debug-module
        modules.net-ctrl-module
      ];

      services.debugging.enable = debugging;

      environment.systemPackages = [ curl ];

      services.netCtrl = {
        enable = true;
        port = netCtrlPort;
        openFirewall = true;
        inherit inventory;
      };
    };
  };

  skipLint = debugging;

  testScript = ''
    import json

    start_all()
    server.wait_for_unit("net-ctrl.service")

    server.wait_for_open_port(${builtins.toString netCtrlPort})

    expected = json.loads('${builtins.toJSON inventory}')

    actual = json.loads(
        server.succeed(
            "${curl}/bin/curl --fail --silent http://localhost:${builtins.toString netCtrlPort}/switches"
        )
    )

    assert expected == actual, "/switches query returns expected content"

  '';
}
