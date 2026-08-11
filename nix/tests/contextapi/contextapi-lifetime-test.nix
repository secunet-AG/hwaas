# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{
  testers,
  lib,
  httpie,
  context-api-url-version-prefix,
  modules,
}:
let
  ctxPort = "8080";
  rsd = import ./rsd.nix;
in
testers.runNixOSTest {
  name = "contextapi-lifetime-test";

  node.specialArgs = { inherit modules; };

  nodes = {
    sut = _: {
      imports = [
        modules.contextapi-module
        ./test-modules/test-config.nix
        ./test-modules/mock-contextapi-satellite-rest-services.nix
        ./test-modules/mock-remote-usb.nix
      ];

      context-api-test-config.enable = true;

      services = {
        # mimic a netctrl instance (enables /power/reset)
        mock-contextapi-satellite-rest-services.enable = true;
        contextApi = {
          enable = true;
          openFirewall = true;
          port = lib.toInt ctxPort;
        };
        # mimic a remote-usb instance (enables /usb/reset)
        mock-remote-usb.enable = true;
      };

      environment.systemPackages = [ httpie ];
    };
  };

  testScript = ''
    import json

    start_all()
    sut.wait_for_unit("context-api.service")
    base_url = "http://localhost:${ctxPort}/${context-api-url-version-prefix}/contexts"

    rsd_object_string = json.dumps(${rsd})
    context_uuid = sut.succeed(f"http --check-status POST {base_url} <<<'{rsd_object_string}'")

    def verify_lifetime(min, max):
      response = sut.succeed(f"http --check-status GET {base_url}/{context_uuid}")
      lifetime = json.loads(response).get("lifetime", 0)
      assert lifetime > min and lifetime < max, f"expected {min} < lifetime < {max}, but got lifetime {lifetime} in response {response}"

    with subtest("verify starting lifetime is ~3600"):
      verify_lifetime(3590, 3600)

    with subtest("verify extending lifetime works"):
      sut.succeed(f"http --check-status PATCH {base_url}/{context_uuid} <<<'{{ \"lifetime\": 3800 }}'")
      verify_lifetime(3790, 3800)

    with subtest("verify extending to something larger than max lifetime fails"):
      # using default value from Rust code: `ContextMaxLifetimeSetting(7_200)`
      response = sut.fail(f"http --check-status PATCH {base_url}/{context_uuid} <<<'{{ \"lifetime\": 7200 }}'")
      assert response == "Cannot extend context lifetime timeout beyond maximum", f"expected error, got {response}"
      # verify no changes happened to the current lifetime as well
      verify_lifetime(3780, 3800)
  '';
}
