# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ pkgs, hwaas }:
let
  ws-proxy = hwaas.packages."${pkgs.system}".ws-proxy-client;
in
rec {
  hwaasTestBase = ./hwaas-test-base.nix;
  hwaasTestLegacyBios = ./hwaas-test-legacy-bios.nix;

  hwaasTestOptions = ./hwaas-test-options;
  hwaasTestAPI = ./hwaas-test-options/check.nix;

  hwaasTestVm = import ./hwaas-test-vm.nix wsProxyClient;

  wsProxyClient = import ./ws-proxy-client.nix ws-proxy;

  machines = {
    legacyBox = import ./legacyBox.nix { inherit hwaasTestLegacyBios; };
  };
}
