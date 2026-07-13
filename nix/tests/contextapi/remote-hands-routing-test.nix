# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ testers
, lib
, httpie
, context-api-url-version-prefix
, modules
,
}:
let
  ctxPort = "8080";
  rsd = import ./rsd.nix;
in
testers.runNixOSTest {
  name = "remote-hands-routing-test";
  node.specialArgs = { inherit modules; };
  nodes = {
    sut =
      { config, ... }:
      {
        imports = [
          ./test-modules/test-config.nix
          ./test-modules/mock-contextapi-satellite-rest-services.nix
          ./test-modules/mock-remote-usb.nix
          modules.contextapi-module
        ];

        context-api-test-config = {
          enable = true;
        };
        services = {
          mock-remote-usb.enable = true;

          contextApi = {
            enable = true;
            openFirewall = true;
            port = lib.toInt ctxPort;
          };

          # mimic a netctrl instance
          mock-contextapi-satellite-rest-services.enable = true;
        };
        environment.systemPackages = [ httpie ];

        # overwriting context-api to wait for the echo server
        systemd.services.context-api = {
          requires = [ "echo-server.service" ];
          after = [ "echo-server.service" ];
        };
      };
  };

  testScript = ''
    import json

    start_all()
    sut.wait_for_unit("context-api.service")
    base_url = "http://localhost:${ctxPort}/${context-api-url-version-prefix}/contexts"

    rsd_object_string = json.dumps(${rsd})
    context_uuid = sut.succeed(f"http --check-status POST {base_url} <<<'{rsd_object_string}'")
    machine_address = f"{base_url}/{context_uuid}/machines/abmr"

    response = sut.succeed(f"http --check-status PUT {machine_address}/power")
    responseJson = json.loads(response)
    assert responseJson["method"] == "PUT"

    sut.succeed(f"http --check-status DELETE {machine_address}/usb")
    sut.succeed(f"http --check-status DELETE {machine_address}/power")
    resetResponse = sut.succeed(f"http --check-status POST {machine_address}/power/reset")
    resetResponseJson = json.loads(resetResponse)
    assert resetResponseJson["path"] == "/power/reset"

    # Due to using f-string, we need to escape the json {} via duplication each
    # Also using """ here so that we can keep the escaping of the quotation marks to a minimum
    putResponse = sut.succeed(f"""http --check-status PUT {machine_address}/usb <<< '[{{"type":"keyboard"}}]' """)
    putResponseJson = json.loads(putResponse)
    assert putResponseJson[0] == { "type": "keyboard" }

    sut.succeed(f"http --check-status GET {machine_address}/usb")
    sut.succeed(f"http --check-status POST {machine_address}/usb/keyboard/text input='Hello World!' newline:=true")
    sut.succeed(f"http --check-status POST {machine_address}/usb/reset")
  '';
}
