# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ ... }:

{
  flake.lib.hwaasTest =
    pkgs:
    import ../../../user-tooling/packages/hwaas-integration-test/default.nix {
      inherit pkgs;
    };
}
