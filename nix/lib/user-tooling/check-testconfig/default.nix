# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

# Checks whether a given attribute set satisfies certain conditions, e.g. it
# has some necessary attributes and no conflicting values.
{ lib
}:
testConfig:
# lib.deepSeq makes sure that all attributes are checked.
lib.deepSeq (lib.evalModules {
  modules = [
    ../../../modules/user-tooling/hwaas-test-options/check.nix
    { config = testConfig; }
  ];
}).config
  testConfig
