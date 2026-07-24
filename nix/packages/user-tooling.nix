# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ pkgs, ... }:

rec {
  user-tooling-benchmarkDataCollector = pkgs.callPackage ../../user-tooling/packages/benchmark-data-collector { };
  user-tooling-hwaasDataUpload = pkgs.callPackage ../../user-tooling/packages/data-upload { };
  user-tooling-hwaasPythonDriver = pkgs.callPackage ../../user-tooling/packages/hwaas-driver { benchmarkDataCollector = user-tooling-benchmarkDataCollector; };
  user-tooling-hwaasTimer = pkgs.python3Packages.callPackage ../../user-tooling/packages/hwaas-timer { };
}
