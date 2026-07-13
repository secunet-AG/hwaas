# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ testers
, httpie
, jq
, modules
,
}:
testers.nixosTest {
  name = "integration-test";
  nodes = {
    server = {
      imports = [
        modules.aruba-switch-mock
      ];

      environment.systemPackages = [
        httpie
        jq
      ];

      services.arubaDummySwitch = {
        enable = true;
        port = 80;
      };

    };
  };

  testScript = ''
    #import json

    start_all()
    server.wait_for_unit("aruba-dummy-switch.service")

    res = server.succeed("${httpie}/bin/http http://localhost/")

    assert res == "<h1>Hello, World!</h1>", "Output mismatch"

  '';
}
