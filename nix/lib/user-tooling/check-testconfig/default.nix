# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ lib }:

/**
  Validate a HWaaS test configuration attribute set.

  Asserts that required attributes are present and don't conflict each other.

  # Inputs

  `testConfig`
  : The HWaaS test configuration to validate.

  # Type

  ```
  AttrSet -> AttrSet
  ```
*/
testConfig:
# lib.deepSeq makes sure that all attributes are checked.
lib.deepSeq
  (lib.evalModules {
    modules = [
      ../../../modules/user-tooling/hwaas-test-options/check.nix
      { config = testConfig; }
    ];
  }).config
  testConfig
