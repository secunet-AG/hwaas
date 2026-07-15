# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

_: {
  imports = [
    ./ws-proxy-client.nix
    ./regen-expected-oas.nix
    ./generate-net-ctrl-client.nix
    ./serve-docs.nix
    ./add-SPDX-license-header.nix
    ./lint-SPDX-license-header.nix
  ];
}
