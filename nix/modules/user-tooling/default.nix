# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ moduleWithSystem, ... }:

rec {
  user-tooling-hwaasTestBase = ./hwaas-test-base.nix;
  user-tooling-hwaasTestLegacyBios = ./hwaas-test-legacy-bios.nix;
  user-tooling-hwaasTestOptions = ./hwaas-test-options;
  user-tooling-hwaasTestAPI = ./hwaas-test-options/check.nix;

  user-tooling-wsProxyClient = moduleWithSystem (
    { config, ... }: import ./ws-proxy-client.nix config.packages.ws-proxy-client
  );

  user-tooling-hwaasTestVm = import ./hwaas-test-vm.nix user-tooling-wsProxyClient;
  user-tooling-machines-legacyBox = import ./legacyBox.nix {
    hwaasTestLegacyBios = user-tooling-hwaasTestLegacyBios;
  };
}
