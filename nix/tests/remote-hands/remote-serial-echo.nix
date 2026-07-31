# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ testers
, modules
,
}:
let
  port = 8080;

  serialConfig.serials.test = {
    type = "stdio";
    command = "cat";
  };

  serviceName = "remote-serial";
in
testers.nixosTest {
  name = "remote-serial-echo-test";

  nodes = {
    sut =
      { pkgs, ... }:
      {
        imports = [
          modules.remote-serial
        ];

        services.remote-serial = {
          enable = true;
          inherit port;
          configFile = builtins.toFile "remote-serial.json" (builtins.toJSON serialConfig);
        };
        environment.systemPackages = with pkgs; [
          httpie
          websocat
          util-linux
        ];
      };
  };

  testScript = ''
    start_all()
    sut.wait_for_unit("${serviceName}.service")
    base_url = "http://localhost:${toString port}/serial"
    ws_url = "ws://127.0.0.1:${toString port}/serial/test/websocket"

    with subtest("get ids"):
      response = sut.succeed(f"http --check-status GET {base_url}")
      assert response == '["test"]'

    with subtest("write and read serial"):
      sut.succeed(f"http --check-status POST {base_url}/test <<<\"Hello World\"")
      response = sut.succeed(f"http --check-status GET {base_url}/test")
      assert response == "Hello World\n"

    with subtest("websocket"):
      response = sut.succeed(f"echo Hello Websocket | websocat --no-close --one-message {ws_url}")
      assert response == "Hello Websocket\n"

    with subtest("delete buffer"):
      sut.succeed(f"http --check-status POST {base_url}/test <<<\"Hello World\"")
      sut.succeed(f"http --check-status DELETE {base_url}/test")
      response = sut.succeed(f"http --check-status GET {base_url}/test")
      assert response == ""

    with subtest("reset"):
      sut.succeed(f"http --check-status POST {base_url}/test <<<\"Hello World\"")
      sut.succeed(f"http --check-status POST {base_url}/reset")
      response = sut.succeed(f"http --check-status GET {base_url}/test")
      assert response == ""

    with subtest("termination"):
      # must be running
      sut.succeed("systemctl is-active ${serviceName}.service")

      # stop the service
      sut.succeed("systemctl stop ${serviceName}.service")

      # Needs to become inactive
      #sut.wait_until_succeeds("systemctl is-active ${serviceName}.service | grep inactive")

      # be strict: "inactive" not failed
      sut.succeed("test \"$(systemctl is-active ${serviceName}.service)\" = inactive")

      # Optional: no process in CGroup
      sut.succeed("test -z \"$(systemctl show -p ControlGroup --value ${serviceName}.service)\" || true")
      sut.succeed("systemctl show -p ControlGroup --value ${serviceName}.service | xargs -I{} sh -c 'test ! -e /sys/fs/cgroup{} || test -z \"$(ls -A /sys/fs/cgroup{})\"'")

      # Optional: check if main PID is gone
      sut.succeed("pid=$(systemctl show -p MainPID --value ${serviceName}.service); test \"$pid\" = 0 || ! kill -0 \"$pid\" 2>/dev/null")

  '';
}
