# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ lib, runCommand }:

{ name, tests }:
let
  evaluatedTests =
    (lib.evalModules {
      modules = [
        ./options.nix
        { config = { inherit tests; }; }
      ];
    }).config;
in
runCommand name { } ''
  echo "${evaluatedTests.text}" > $out

  ${lib.optionalString (!evaluatedTests.passed) ''
    echo "${evaluatedTests.text}"
    exit 1
  ''}
''
