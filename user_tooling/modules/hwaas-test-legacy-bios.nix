# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ lib, modulesPath, ... }:
{
  imports = [
    ./hwaas-test-base.nix
    "${modulesPath}/installer/cd-dvd/installation-cd-minimal.nix"
  ];

  image.baseName = lib.mkForce "hwaas";
}
