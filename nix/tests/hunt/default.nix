# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

_: {
  perSystem =
    { pkgs, self', ... }:
    {
      checks.hunt-rust-log = pkgs.callPackage ./rust-log.nix {
        inherit (self'.packages) hunt;
      };
    };
}
