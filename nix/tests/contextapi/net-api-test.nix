# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

# Test if the generated NetCtrl client behaves like expected
{ testers
, context-api-url-version-prefix
, httpie
, jq
, modules
,
}:
let
  switch = "switch1";
  net_ctrl_port = "4455";
  port = 8080;
  switch_connection_1 = "lan1";
  rsd = import ./rsd.nix;
in
testers.runNixOSTest {
  name = "net-api-test";
  node.specialArgs = { inherit modules; };
  nodes = {
    sut = {

      imports = [
        ./test-modules/test-config.nix
        ./test-modules/mock-contextapi-satellite-rest-services2.nix
        ./test-modules/mock-remote-usb.nix
        modules.contextapi-module
        modules.net-ctrl-module
      ];

      context-api-test-config = {
        inherit switch net_ctrl_port switch_connection_1;
      };

      context-api-test-config = {
        enable = true;
        networkIdsEnd = 6; # this test try to reserve 3 networks
      };

      services = {
        mock-contextapi-satellite-rest-services.enable = true;
        mock-remote-usb.enable = true;
        contextApi = {
          enable = true;
          inherit port;
          openFirewall = true;
        };
      };

      environment.systemPackages = [
        httpie
        jq
      ];
    };
  };

  testScript = ''
    import json

    start_all()
    sut.wait_for_unit("net-ctrl.service")

    # This test assumes the NetCtrl knows the switch
    res = json.loads(sut.succeed("http --check-status GET http://127.0.0.1:${net_ctrl_port}/switches"))
    assert "${switch}" in res, "switch not available"

    sut.wait_for_unit("context-api.service")

    # Test handle ports
    url_base = "http://127.0.0.1:${builtins.toString port}/${context-api-url-version-prefix}"

    context_reservation_url = f"{url_base}/contexts"

    rsd_object_string = json.dumps(${rsd})
    context_uuid = sut.succeed(f"http --check-status POST {context_reservation_url} <<<'{rsd_object_string}'")

    def get_state(url: str):
      return json.loads(sut.succeed(f"http --check-status GET {url}"))

    def get_networks():
      url = f"{url_base}/contexts/{context_uuid}/networks"
      return json.loads(sut.succeed(f"http --check-status GET {url}"))

    body = "{\"abmr\": {\"${switch_connection_1}\" : {}}}"
    body_json = json.loads(body)

    # Test if the same request is allowed and results in the same setup
    url = f"{url_base}/contexts/{context_uuid}/networks/neta"
    sut.succeed(f"http --check-status PUT {url} <<<'{body}'")
    sut.succeed(f"http --check-status PUT {url} <<<'{body}'")
    assert body_json == get_state(url), "State check failed: neta has not the expected assigment"

    # Test if network deletions works
    sut.succeed(f"http --check-status DELETE {url}")
    assert "neta" not in get_networks(), "State check failed: neta not deleted"

    # Create a new network with the previous asigment
    url = f"{url_base}/contexts/{context_uuid}/networks/netb"
    sut.succeed(f"http --check-status PUT {url} <<<'{body}'")
    assert body_json == get_state(url), "State check failed: netb has not the expected assigment"

    # Test if live re-assigment works and the netb's network state is gone
    url_last = url
    url = f"{url_base}/contexts/{context_uuid}/networks/netc"
    sut.succeed(f"http --check-status PUT {url} <<<'{body}'")
    assert body_json == get_state(url), "State check failed: netc has not assigment"
    assert {} == get_state(url_last), "State check failed: netb not empty"

    # Delete all remaining networks
    sut.succeed(f"http --check-status DELETE {url}") # netc
    sut.succeed(f"http --check-status DELETE {url_last}") # netb
    assert len(get_networks()) == 0, "State check failed: networks left"

  '';
}
