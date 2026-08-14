# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ lib, config, ... }:
let
  assertionOptions = {
    check = lib.mkOption {
      type = lib.types.unspecified;
      description = ''
        The check itself. Will be evaluated with `builtins.tryEval`.
      '';
    };
    expected = lib.mkOption {
      type = lib.types.bool;
      description = ''
        The expected value of `(builtins.tryEval check).success`.
      '';
    };
    message = lib.mkOption {
      type = lib.types.str;
      description = ''
        A description of what this test checks.
      '';
    };
  };

  resultOptions = {
    result = lib.mkOption { type = lib.types.bool; };
    message = lib.mkOption { type = lib.types.str; };
  };
in
{
  options = {
    tests = lib.mkOption {
      type =
        with lib.types;
        listOf (submodule {
          options = assertionOptions;
        });
    };

    results = lib.mkOption {
      internal = true;
      type =
        with lib.types;
        listOf (submodule {
          options = resultOptions;
        });
      description = ''
        The results of the evaluated tests.
      '';
      default = builtins.map (
        {
          check,
          expected,
          message,
        }:
        {
          result = (builtins.tryEval check).success == expected;
          inherit message;
        }
      ) config.tests;
    };

    passed = lib.mkOption {
      internal = true;
      type = lib.types.bool;
      description = ''
        Whether all tests passed.
      '';
      default = builtins.all ({ result, ... }: result) config.results;
    };

    text = lib.mkOption {
      internal = true;
      type = lib.types.str;
      description = ''
        A textual representation of the test results.
      '';
      default =
        let
          maxLen = lib.lists.fold lib.trivial.max (-1) (
            builtins.map ({ message, ... }: builtins.stringLength message) config.results
          );
          desiredLen = maxLen + 1;
          padRight = msg: msg + lib.fixedWidthString (desiredLen - (builtins.stringLength msg)) " " "";
        in
        lib.concatMapStringsSep "\n" (
          { result, message }: (padRight message) + (if result then "success" else "failure")
        ) config.results;
    };
  };
}
