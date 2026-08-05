# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ testers, modules }:
testers.runNixOSTest {
  name = "contextapi-startup-test";

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
        };
        # mimic a remote-usb instance (enables /usb/reset)
        mock-remote-usb.enable = true;
      };
    };
  };

  testScript = ''
    start_all()
    sut.wait_for_unit("context-api.service")
  '';
}
