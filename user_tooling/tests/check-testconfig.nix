# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

# Contains a few simple checks for the checkTestconfig function.
{ lib
, runCommand
}:
let
  checkTestconfig = import ../lib/check-testconfig { inherit lib; };
  nixUnittest = import ../lib/nix-unittest { inherit lib runCommand; };

  goodConfig = {
    name = "Hello World Test";
    apiUrl = "https://api.hwaas.placeholder.com/v5";
    machines.bmr1 = {
      image = "linux.img";
      platform = "bmrType1";
    };
    networks.network1 = [
      { machine = "bmr1"; networkInterfaces = [ "LAN1" ]; }
    ];
    testScript = { ... }: ''
      execute_tests()
    '';
  };

  machineConfig = { lib, ... }: { stateVersion = lib.trivial.release; };

  mkTest = { expected, config, message }: { check = checkTestconfig config; inherit message expected; };
  testSuccess = attrs: mkTest ({ expected = true; } // attrs);
  testFailure = attrs: mkTest ({ expected = false; } // attrs);
in
nixUnittest
{
  name = "checkTestconfig-test";
  tests = [
    (testSuccess { config = goodConfig; message = "Valid config evaluates without error"; })

    (testFailure { config = goodConfig // { machines.bmr1.image = null; }; message = "Machine needs either image or config"; })
    (testFailure { config = goodConfig // { machines.bmr1.config = machineConfig; }; message = "Machine can't have image and config"; })

    (testFailure { config = goodConfig // { machines.bmr1 = { image = null; config = machineConfig; }; }; message = "Using machine.config is currently not possible"; })

    (testFailure { config = lib.filterAttrs (n: _: n == "name" goodConfig); message = "Config needs a name"; })
    (testFailure { config = lib.filterAttrs (n: _: n == "machines" goodConfig); message = "Config needs machines"; })
    (testFailure { config = lib.filterAttrs (n: _: n == "networks" goodConfig); message = "Config needs networks"; })
    (testFailure { config = lib.filterAttrs (n: _: n == "testScript" goodConfig); message = "Config needs a testScript"; })
    (testFailure { config = lib.filterAttrs (n: _: n == "apiUrl" goodConfig); message = "Config needs a apiUrl"; })
  ];
}
