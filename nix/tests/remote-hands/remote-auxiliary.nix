# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ testers
, modules
,
}:
let
  port = 8080;
  auxMockPort = 12345;

  mkAuxDevice = device_id: cmd: {
    config = {
      id = device_id;
      url = "http://localhost:${builtins.toString auxMockPort}";
      inherit cmd;
    };
  };
  echoCmd = "echo Activation: $1";
  auxiliaryConfig.devices = {
    aux1 = mkAuxDevice "aux1" echoCmd;
    aux2 = mkAuxDevice "aux2" echoCmd;
    aux3 = mkAuxDevice "aux3" echoCmd;
    auxX = mkAuxDevice "auxX" "if $1; then exit 1; else ${echoCmd}; fi";
  };

in
testers.nixosTest {
  name = "remote-auxiliary-test";

  nodes = {
    sut =
      { pkgs, ... }:
      {
        imports = [
          modules.remote-auxiliary
          modules.test-restapi-echo-server
        ];

        services.remote-auxiliary = {
          enable = true;
          inherit port;
          configFile = builtins.toFile "remote-auxiliary.json" (builtins.toJSON auxiliaryConfig);
        };
        environment.systemPackages = with pkgs; [
          httpie
          util-linux
        ];
        systemd.services.remote-auxiliary.after = [ "echo-server.service" ];

        services.http-echo-server = {
          enable = true;
          port = auxMockPort;
        };
      };
  };

  testScript = ''
    import json
    import urllib.parse

    def get_auxiliary_dev_request():
      return json.loads(sut.succeed(
        "journalctl -u echo-server.service "
        "| tail -n1 "
        "| grep -oP '\{.*\}'"
        ))

    start_all()
    sut.wait_for_unit("remote-auxiliary.service")
    base_url = "http://localhost:${toString port}"

    with subtest("GET all"):
      response = json.loads(sut.succeed(f"http --check-status GET {base_url}/auxiliaries"))
      ids = [device["id"] for device in response]
      ids.sort()
      assert ids == ["aux1", "aux2", "aux3", "auxX"], f"GET all API failed, returned {ids}"

    with subtest("PUT and GET activation"):
      sut.succeed(f"http --check-status --raw 'true' PUT {base_url}/auxiliaries/aux1/activation")
      sut.succeed(f"http --check-status --raw 'false' PUT {base_url}/auxiliaries/aux2/activation")
      sut.succeed(f"http --check-status --raw 'true' PUT {base_url}/auxiliaries/aux3/activation")
      assert json.loads(sut.succeed(f"http --check-status GET {base_url}/auxiliaries/aux1"))['activation'] == True
      assert json.loads(sut.succeed(f"http --check-status GET {base_url}/auxiliaries/aux2"))['activation'] == False
      assert json.loads(sut.succeed(f"http --check-status GET {base_url}/auxiliaries/aux3"))['activation'] == True

      sut.succeed(f"http --check-status --raw 'false' PUT {base_url}/auxiliaries/aux1/activation")
      sut.succeed(f"http --check-status --raw 'true' PUT {base_url}/auxiliaries/aux2/activation")
      sut.succeed(f"http --check-status --raw 'false' PUT {base_url}/auxiliaries/aux3/activation")
      assert json.loads(sut.succeed(f"http --check-status GET {base_url}/auxiliaries/aux1/activation")) == False
      assert json.loads(sut.succeed(f"http --check-status GET {base_url}/auxiliaries/aux2/activation")) == True
      assert json.loads(sut.succeed(f"http --check-status GET {base_url}/auxiliaries/aux3/activation")) == False

    with subtest("reverse proxy"):
      sut.succeed(f"http --check-status GET {base_url}/auxiliaries/aux1/api")
      sut.succeed(f"http --check-status GET {base_url}/auxiliaries/aux1/api/test")
      sut.succeed(f"http --check-status --raw 'true' PUT {base_url}/auxiliaries/aux1/api")
      sut.succeed(f"http --check-status --raw 'false' POST {base_url}/auxiliaries/aux1/api/post")
      sut.succeed(f"http --check-status DELETE {base_url}/auxiliaries/aux1/api")

      auxiliaryPath = "/foo/bar"
      method = "PUT"
      body = "my body"
      headerName = "custom-header"
      headerValue = "my-header"
      queryParam1 = "my_param"
      queryValue1 = "1"
      queryParam2 = "hello"
      queryValue2 = "world"

      sut.succeed(f'http --check-status --raw "{body}" {method} ' +
        f'{base_url}/auxiliaries/aux1/api{auxiliaryPath}?{queryParam1}={queryValue1}\&{queryParam2}={queryValue2} {headerName}:{headerValue}'
      )

      result = get_auxiliary_dev_request()

      assert result["method"] == method, f"Expected {method} but was {result['method']}"
      assert result["path"] == auxiliaryPath , f"Expected {auxiliaryPath} but was {result['path']}"
      assert result["body"] == body, f"Expected request body '{body}' but was {result['body']}"
      assert result["headers"].get(headerName) == headerValue, f"Expected custom header but was not found or did not match: {result['headers']}"
      assert result["query"].get(queryParam1) == [queryValue1], f"Expected first query param but was not found or did not match: {result['query']}"
      assert result["query"].get(queryParam2) == [queryValue2], f"Expected second query param but was not found or did not match: {result['query']}"

    with subtest("query parameter with escaped &"):
      queryParam3 = "my_&_param"
      queryValue3 = "1&2"
      encodedQueryParam = urllib.parse.quote(queryParam3)
      encodedQueryValue = urllib.parse.quote(queryValue3)

      sut.succeed(f'http --check-status --raw "" GET {base_url}/auxiliaries/aux1/api?{encodedQueryParam}={encodedQueryValue}')
      result = get_auxiliary_dev_request()

      assert result["query"].get(queryParam3) == [queryValue3], f"Expected query param but was not found or did not match: {result['query']}"

    with subtest("error codes"):
      # 404 Not Found
      stdout1 = sut.fail(f"http --check-status GET {base_url}/auxiliaries/aux4/activation 2>&1")
      assert "404 Not Found" in stdout1, f"Expected 404, got: {stdout1}"
      stdout2 = sut.fail(f"http --check-status --raw 'true' PUT {base_url}/auxiliaries/aux4/activation 2>&1")
      assert "404 Not Found" in stdout2, f"Expected 404, got: {stdout2}"
      stdout3 = sut.fail(f"http --check-status --raw 'hello' PUT {base_url}/auxiliaries/aux4/activation 2>&1")
      assert "404 Not Found" in stdout3, f"Expected 404, got: {stdout3}"
      stdout4 = sut.fail(f"http --check-status GET {base_url}/auxiliaries/aux4/api 2>&1")
      assert "404 Not Found" in stdout4, f"Expected 404, got: {stdout4}"

      # 400 Bad Request
      stdout5 = sut.fail(f"http --check-status --raw 'hello' PUT {base_url}/auxiliaries/aux3/activation 2>&1")
      assert "400 Bad Request" in stdout5, f"Expected 400, got: {stdout5}"

      # 500 Internal Server Error
      stdout6 = sut.fail(f"http --check-status --raw 'true' PUT {base_url}/auxiliaries/auxX/activation 2>&1")
      assert "500 Internal Server Error" in stdout6, f"Expected 500, got: {stdout6}"

    with subtest("reset API"):
      # Preparation: verify that at least one interface is activated
      response = json.loads(sut.succeed(f"http --check-status GET {base_url}/auxiliaries"))
      activation = [device["activation"] for device in response]
      assert any(activation) == True, "at least one interface should be activated"

      # Test case
      response = json.loads(sut.succeed(f"http --check-status POST {base_url}/auxiliaries/reset"))
      activation = [device["activation"] for device in response]
      assert all(activation) == False, "all interfaces should be deactivated"
  '';
}
