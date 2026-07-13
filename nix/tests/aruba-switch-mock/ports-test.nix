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
  name = "ports-test";
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
    import json

    start_all()
    server.wait_for_unit("aruba-dummy-switch.service")

    def login_req():
      login = json.dumps({"userName": "hwaas", "password": "hwaas"})
      return f"http --check-status --body POST http://127.0.0.1:80/rest/v1/login-sessions <<<'{login}'"

    # login
    res_cookie = server.succeed(login_req())
    cookie = json.loads(res_cookie)["cookie"]

    # get ports
    ports = json.loads(server.succeed(f"http --check-status http://127.0.0.1:80/rest/v1/ports Cookie:{cookie}"))
    ports = ports["port_element"]
    assert len(ports) == 16, "Number of Ports mismatch"
  '';
}
