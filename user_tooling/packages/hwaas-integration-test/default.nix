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
  inherit (builtins) isFunction readFile removeAttrs toJSON;
  inherit (pkgs) lib;

  helpers = import ./helpers.nix { inherit lib; };

  checkTestconfig = import ../../lib/check-testconfig { inherit lib; };
in
helpers // {
  inherit checkTestconfig;

  filterTestConfig =
    hwaasTestConfig:
    let
      # Attributes we need to filter before passing the test configuration to
      # the nixosTest function. Otherwise, we would get an error about
      # unexpected parameters.
      hwaasConfigOnlyAttributes = [ "machines" "networks" "apiUrl" ];
    in
    removeAttrs hwaasTestConfig hwaasConfigOnlyAttributes;

  mkTestConfig =
    { extraPythonPackages ? _: [ ], ... }@hwaasTestConfig:
    let
      benchmarkDataCollector = pkgs.callPackage ../benchmark-data-collector { };
      hwaasPythonDriver = pkgs.callPackage ../hwaas-driver { inherit benchmarkDataCollector; };
    in
    hwaasTestConfig // {
      extraPythonPackages =
        pyPkgs:
        [
          hwaasPythonDriver
          benchmarkDataCollector
        ] ++ extraPythonPackages pyPkgs;
    };

  mkTestScript =
    { name
    , testScript
    , machines ? { }
    , networks ? { }
    , apiUrl
    , ...
    }:
    testScriptArgs:
    let
      hwaasConfig =
        let
          inherit (helpers) mapNetworks;

          config = checkTestconfig {
            # Attributes required and checked by our HWaaS test configuration checker.
            inherit machines name networks testScript apiUrl;
          };

          config' = {
            inherit (config) machines apiUrl;
            networks = mapNetworks config.networks;
          };
        in
        "'''${toJSON config'}'''";

      testScript' =
        if isFunction testScript then
          testScript testScriptArgs
        else
          testScript;
    in
    ''
      HWAAS_CONFIG = ${hwaasConfig}

      ${readFile ./setup.py}

      ${testScript'}
    '';

  mkTest =
    let
      standardNixosTestConfig = { lib, ... }: {
        config.nixpkgs.pkgs = lib.mkDefault pkgs;
      };
    in
    (import "${pkgs.path}/nixos/lib/testing-python.nix" {
      inherit (pkgs.stdenv.hostPlatform) system;
      inherit pkgs;
      extraConfigurations = [
        standardNixosTestConfig
      ];
    }).simpleTest;

  __functor =
    { filterTestConfig, mkTest, mkTestConfig, mkTestScript, ... }:
    hwaasTestConfig:
    let
      testConfig = mkTestConfig (filterTestConfig hwaasTestConfig) // {
        testScript = mkTestScript hwaasTestConfig;
      };

      test = mkTest testConfig;
    in
    lib.recursiveUpdate test {
      meta.tag = "nix-integration-test";
    };
}
