# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ testers
, modules
,
}:
let
  port = 8080;

  mkControl = filename: {
    type = "custom";
    config = {
      on = "echo on > ${filename}";
      off = "echo off > ${filename}";
      reset = "echo reset > ${filename}";
      query = "cat ${filename}";
    };
  };

  powerConfig.controls = {
    pow1 = mkControl "/tmp/pow1";
    pow2 = mkControl "/tmp/pow2";
    pow3 = mkControl "/tmp/pow3";
  };

in
testers.nixosTest {
  name = "remote-power-test";

  nodes = {
    sut =
      { pkgs, ... }:
      {
        imports = [
          modules.remote-power
        ];

        services.remote-power = {
          enable = true;
          inherit port;
          configFile = builtins.toFile "remote-power.json" (builtins.toJSON powerConfig);
        };
        environment.systemPackages = with pkgs; [
          httpie
          util-linux
        ];
      };
  };

  testScript = ''
    import json

    start_all()
    sut.wait_for_unit("remote-power.service")
    base_url = "http://localhost:${toString port}"

    # use PUT and DELETE to turn on and off individual interfaces, GET to query state
    sut.succeed(f"http --check-status PUT {base_url}/power/pow1")
    sut.succeed(f"http --check-status DELETE {base_url}/power/pow2")
    sut.succeed(f"http --check-status PUT {base_url}/power/pow3")
    assert json.loads(sut.succeed(f"http --check-status GET {base_url}/power/pow1"))['state'] == True
    assert json.loads(sut.succeed(f"http --check-status GET {base_url}/power/pow2"))['state'] == False
    assert json.loads(sut.succeed(f"http --check-status GET {base_url}/power/pow3"))['state'] == True

    sut.succeed(f"http --check-status DELETE {base_url}/power/pow1")
    sut.succeed(f"http --check-status PUT {base_url}/power/pow2")
    sut.succeed(f"http --check-status DELETE {base_url}/power/pow3")
    assert json.loads(sut.succeed(f"http --check-status GET {base_url}/power/pow1"))['state'] == False
    assert json.loads(sut.succeed(f"http --check-status GET {base_url}/power/pow2"))['state'] == True
    assert json.loads(sut.succeed(f"http --check-status GET {base_url}/power/pow3"))['state'] == False

    # test RESET ALL & GET ALL
    sut.succeed(f"http --check-status POST {base_url}/power/reset")
    power = json.loads(sut.succeed(f"http --check-status GET {base_url}/power"))
    assert len(power) == 3
    interface_names = [p['power_id'] for p in power]
    interface_names.sort()
    assert interface_names == ['pow1', 'pow2', 'pow3']
    assert [p['state'] for p in power] == [False, False, False]

    # test PUT ALL & DELETE ALL
    sut.succeed(f"http --check-status PUT {base_url}/power")
    power = json.loads(sut.succeed(f"http --check-status GET {base_url}/power"))
    assert [p['state'] for p in power] == [True, True, True]
    sut.succeed(f"http --check-status DELETE {base_url}/power")
    power = json.loads(sut.succeed(f"http --check-status GET {base_url}/power"))
    assert [p['state'] for p in power] == [False, False, False]
  '';
}
