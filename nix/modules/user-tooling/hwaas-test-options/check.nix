# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

/**
  Validate the HWaaS test configuration.

  Note that a (semantically) valid HWaaS test configuration does not mean the tests are guaranteed
  to succeed.

  # Inputs

  `lib`
  : The nixpkgs `lib` with auxiliary functions.

  `config`:
  : The (partial) HWaaS test configuration. In particular, this is expected to have the following
    fields (see also ./default.nix): `machines`

  # Output

  None. If this module evaluates without errors, the HWaaS test configuration is valid.

  # Type

  ```text
  :: AttrSet -> _
  ```
*/
{ lib, config, ... }:
let
  inherit (builtins) filter map;
  inherit (lib)
    assertMsg
    concatLines
    flatten
    mapAttrsToList
    mkOption
    types
    ;

  inherit (config) assertions machines;

  filterAssertions = filter ({ assertion, ... }: !assertion);

  extractMessages = map ({ message, ... }: message);

  checkMachine = name: { image, config, ... }: [
    {
      assertion = !(image != null && config != null);
      message = "Cannot set ${name}.image and ${name}.config!";
    }
    {
      assertion = !(image == null && config == null);
      message = "${name} has no image and no config!";
    }
    {
      assertion = config == null;
      message = "${name} has a config. Using this option is currently not supported!";
    }
  ];

  forEachMachine = machines: flatten (mapAttrsToList checkMachine machines);
in
{
  imports = [ ./. ];

  options = {
    name = mkOption {
      type = types.str;
      description = ''
        The name of this test.
      '';
    };

    assertions = mkOption {
      type = types.listOf types.unspecified;
      internal = true;
      default = forEachMachine machines;
      description = ''
        This option expresses conditions that must hold for the evaluation of
        the test configuration to succeed, along with associated error messages
        for the user.
      '';
    };

    passed = mkOption {
      type = types.bool;
      internal = true;
      default =
        let
          failedAssertions = filterAssertions assertions;
          messages = extractMessages failedAssertions;
        in
        assertMsg (messages == [ ]) (concatLines messages);
      description = ''
        This option is solely to throw an error if any assertion doesn't hold.
      '';
    };
  };
}
