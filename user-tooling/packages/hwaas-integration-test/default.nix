# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

# We evaluate the HWaaS test configuration for type validation. Since our
# validation function is limited to actual added functionality, we filter
# the configuration accordingly. The NixOS integration test has a
# validation function as well, thus we need to filter out the additional
# attributes from the HWaaS test driver to avoid evaluation failures.

{ pkgs }:
let
  inherit (builtins)
    isFunction
    readFile
    toJSON
    ;
  inherit (pkgs) lib;

  helpers = import ./helpers.nix { inherit lib; };

  checkTestconfig = import ../../../nix/lib/user-tooling/check-testconfig { inherit lib; };
in
helpers
// {
  inherit checkTestconfig;

  /**
    Inject runtime dependencies for HWaaS user tooling into a HWaaS test.

    The runtime dependencies are required on the control VM node to execute the underlying HWaaS
    code that extends the builtin Python test driver.

    # Inputs

    `hwaasTestConfig`
    : The full configuration for a HWaaS test.

    # Type

    ```text
    mkTestConfig :: AttrSet -> AttrSet
    ```
  */
  mkTestConfig =
    {
      extraPythonPackages ? _: [ ],
      ...
    }@hwaasTestConfig:
    let
      benchmarkDataCollector = pkgs.callPackage ../benchmark-data-collector { };
      hwaasPythonDriver = pkgs.callPackage ../hwaas-driver { inherit benchmarkDataCollector; };
    in
    hwaasTestConfig
    // {
      extraPythonPackages =
        pyPkgs:
        [
          hwaasPythonDriver
          benchmarkDataCollector
        ]
        ++ extraPythonPackages pyPkgs;
    };

  /**
    Generate the test script for a HWaaS test based on a test configuration.

    Extends the user-provided test script for the NixOS VM test with a custom prelude that
    prearranges various HWaaS conveniences (such as remote machine objects etc.). The test script
    (expected in the `testScript` attribute of the input argument) may be a function taking
    the exact same argument as for a regular NixOS VM test. Please refer to upstream documentation
    for additional information.

    # Inputs

    `...`
    : The full configuration for a HWaaS test. This must include at least the following attributes:
      `name`, `testScript`, `apiUrl`.

    # Output

    A custom, HWaaS-specific test script, that wraps the initial user-provided test script.

    # Type

    ```text
    mkTestScript :: AttrSet -> AttrSet -> String
    ```
  */
  mkTestScript =
    {
      name,
      testScript,
      machines ? { },
      networks ? { },
      apiUrl,
      ...
    }:
    testScriptArgs:
    let
      hwaasConfig =
        let
          inherit (helpers) mapNetworks;

          config = checkTestconfig {
            # Attributes required and checked by our HWaaS test configuration checker.
            inherit
              name
              machines
              networks
              apiUrl
              ;
          };

          config' = {
            inherit (config) machines apiUrl;
            networks = mapNetworks config.networks;
          };
        in
        "'''${toJSON config'}'''";

      testScript' =
        if isFunction testScript then
          testScript (
            testScriptArgs
            || (builtins.warn "ignoring empty 'testScriptArgs' for test script function in HWaaS test '${name}'" null)
          )
        else
          (builtins.warn "ignoring input 'testScriptArgs' for non-function test script in HWaaS test '${name}'" testScript);
    in
    ''
      HWAAS_CONFIG = ${hwaasConfig}

      ${readFile ./setup.py}

      ${testScript'}
    '';

  __functor =
    { mkTestConfig, mkTestScript, ... }:
    hwaasTestConfig:
    pkgs.testers.runNixOSTest {
      imports = [
        "${../../../nix/modules/user-tooling/hwaas-test-options/default.nix}"
        (
          (mkTestConfig hwaasTestConfig)
          // {
            testScript = lib.mkForce (lib.traceVal (mkTestScript hwaasTestConfig));
          }
        )
        ({ lib, ... }: { config.hostPkgs = lib.mkDefault pkgs; })
      ];
    };
}
