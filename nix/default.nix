# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

_: {
  imports = [
    ./git-hooks.nix
    ./shell.nix
    ./crane.nix
    ./modules
    ./tests
    ./apps
    ./overlay.nix
    ./packages
    ./ci
  ];
}
