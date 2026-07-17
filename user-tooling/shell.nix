# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ ... }:
let
  sources = import ./nix/sources.nix { };
  pkgs = import sources.nixpkgs { };
  defaultPackages = import ./default.nix { inherit pkgs; system = builtins.currentSystem; };
in
pkgs.mkShell {
  buildInputs = with pkgs; [
    niv
    nixpkgs-fmt
    # For manual tests with the HWaaS
    httpie
    # Serial connection client for serial interactions with the HWaaS
    websocat
  ];

  shellHook = ''
    ${defaultPackages.checks.preCommit.shellHook}
  '';
}
