# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ benchmarkDataCollector, pkgs, pre-commit-hooks }:
{
  checkTestconfig = pkgs.callPackage ./check-testconfig.nix { };
  preCommit = pkgs.callPackage ./pre-commit.nix { inherit pre-commit-hooks benchmarkDataCollector; };
}
