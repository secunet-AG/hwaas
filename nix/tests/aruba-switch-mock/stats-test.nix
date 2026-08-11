# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{
  testers,
  httpie,
  jq,
  modules,
}:
testers.nixosTest {
  name = "stats-test";
  nodes = {
    server = {
      imports = [ modules.aruba-switch-mock ];

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

    def get_stats():
      return "http --check-status --body http://127.0.0.1:80/stats"

    # get initial stats
    initial_stats = json.loads(server.succeed(get_stats()))
    assert initial_stats == {"/stats": 1}, "Initial stats mismatch"

    # make some calls
    server.succeed("http http://127.0.0.1:80/")

    # httpie called without '--check-status' because this one will fail
    server.succeed("http http://127.0.0.1:80/rest/v1/")

    # some more login, auth and logout calls
    for i in range(2):
      res_cookie = server.succeed(login_req())
      cookie = json.loads(res_cookie)["cookie"]
      server.succeed(f"http --check-status http://127.0.0.1:80/rest/v1/ Cookie:{cookie}")
      server.succeed(f"http --check-status DELETE http://127.0.0.1:80/rest/v1/login-sessions Cookie:{cookie}")

    # get initial stats
    final_stats = json.loads(server.succeed(get_stats()))
    assert final_stats == {"/rest/v1/login-sessions": 4, "/stats": 2, "/rest/v1/": 3, "/": 1}, "final stats mismatch"

  '';
}
