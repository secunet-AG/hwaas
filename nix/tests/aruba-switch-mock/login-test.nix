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
  name = "login-test";
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
    from typing import List

    start_all()
    server.wait_for_unit("aruba-dummy-switch.service")

    login = json.dumps({"userName": "hwaas", "password": "hwaas"})

    def login_req(login):
      return f"http --check-status --body POST http://127.0.0.1:80/rest/v1/login-sessions <<<'{login}'"

    # Allow 5 sessions in parallel:
    cookies: List[str] = []
    for i in range(5):
      res_cookie = server.succeed(login_req(login))
      cookie = json.loads(res_cookie)["cookie"]
      cookies.append(cookie)
      res = server.succeed(f"http --check-status http://127.0.0.1:80/rest/v1/ Cookie:{cookie}")
      assert res == "<h1>Hello, User:hwaas!</h1>", "Output mismatch"

    # a 6th session MUST NOT be established
    res = server.fail(login_req(login))
    print("fail res: " + res)

    for cookie in cookies:
      server.succeed(f"http --check-status DELETE http://127.0.0.1:80/rest/v1/login-sessions Cookie:{cookie}")

    # another login should now succseed again
    server.succeed(login_req(login))



  '';
}
