# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{
  testers,
  debugging ? false,
  curl,
  hey,
  httpie,
  jq,
  modules,
}:
let
  netCtrlPort = 8080;
  inventory = import ./inventory.nix;
in
testers.nixosTest {
  name = "net-ctrl-api-client-cache";
  nodes = {
    server = {
      imports = [
        modules.test-debug-module
        modules.net-ctrl-module
        modules.aruba-switch-mock
      ];

      services = {
        debugging.enable = debugging;

        netCtrl = {
          enable = true;
          port = netCtrlPort;
          openFirewall = true;
          inherit inventory;
        };

        arubaDummySwitch = {
          enable = true;
          port = 80;
        };
      };

      environment.systemPackages = [
        httpie
        jq
        curl
        hey
      ];

    };
  };

  skipLint = debugging;

  testScript = ''
    import json

    start_all()
    server.wait_for_unit("aruba-dummy-switch.service")
    server.wait_for_unit("net-ctrl.service")

    server.wait_for_open_port(${builtins.toString netCtrlPort})

    # call a route many times concurrently.
    # only the first one should trigger a login.

    # launch load tester with 10 workers for 1 second
    server.succeed("hey -z 1s -c 10 http://127.0.0.1:${builtins.toString netCtrlPort}/switches/switch1")

    # get route call statistics form aruba switch mock
    res = json.loads(server.succeed("curl --fail --silent http://localhost:80/stats"))

    expected_stats = {"/rest/v1/login-sessions": 1, "/rest/v1/ports": 2, "/stats": 1}
    assert res == expected_stats, "Statistics do not match expectation"

  '';
}
