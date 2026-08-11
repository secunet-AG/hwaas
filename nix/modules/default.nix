# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ moduleWithSystem, ... }: {
  flake.nixosModules = {
    aruba-switch-mock = moduleWithSystem (import ./aruba-switch-mock.nix);
    rpi-status-display = moduleWithSystem (import ./rpi-status-display.nix);
    ws-client-module = moduleWithSystem (import ./ws-client-module.nix);
    ws-gateway-module = moduleWithSystem (import ./ws-gateway-module.nix);
    ws-gateway-net-module = import ./ws-gateway-net-module.nix;
    net-ctrl-module = moduleWithSystem (import ./net-ctrl-module.nix);
    remote-serial = moduleWithSystem (import ./remote-serial.nix);
    remote-power = moduleWithSystem (import ./remote-power.nix);
    remote-usb = moduleWithSystem (import ./remote-usb.nix);
    remote-auxiliary = moduleWithSystem (import ./remote-auxiliary.nix);
    contextapi-module = moduleWithSystem (import ./contextapi-module.nix);
    maintainer-cli = moduleWithSystem (import ./maintainer-cli.nix);

    # these modules are only supposed to be used in the HWaaS NixOS integration tests
    test-otel-collector = import ./test-otel-collector.nix;
    test-debug-module = import ./test-debug-module.nix;
    test-restapi-echo-server = import ./test-restapi-echo-server.nix;
    test-http-sim = moduleWithSystem (import ./test-http-sim.nix);
    test-debug-serials = import ./test-debug-serials.nix;

  };
}
