# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ lib, ... }:
let
  inherit (lib) mkOption;
  inherit (lib.types) attrsOf deferredModule either functionTo listOf nullOr str submodule int;

  machineOptions = {
    platform = mkOption {
      type = str;
      description = ''
        The HWaaS machine platform specifier. A list of machines is available
        using
        `curl -X 'GET' 'http://api.hwaas.placeholder.com/v5/inventory' -H 'accept: application/json'`
      '';
    };
    machine_id = mkOption {
      type = nullOr int;
      default = null;
      description = ''
        Request a specific hardware machine by id.

        Warning: Each id can only be used once per test.

        Possible ids can be found in the corresponding documentation.

        It is not recommended to use this option as it can slow down the provision of the tests.
      '';
    };
    image = mkOption {
      type = nullOr str;
      default = null;
      description = ''
        The binary this machine should boot.
      '';
    };
    additionalImages = mkOption {
      type = listOf str;
      default = [ ];
      description = ''
        Additional binaries the machine should mount.
      '';
    };
    config = mkOption {
      type = nullOr deferredModule;
      default = null;
      description = ''
        A NixOS configuration declaring the whole system under test.
        Warning: This option is currently not supported, but will be enabled later.
      '';
    };
  };

  connectionOptions = {
    machine = mkOption {
      type = str;
      description = ''
        The machine name of a machine that is part of this context.
      '';
    };
    networkInterfaces = mkOption {
      type = listOf str;
      description = ''
        A list of network interfaces to connect to this virtual network.
      '';
    };
  };
in
{
  options = {
    apiUrl = mkOption {
      type = str;
      description = ''
        The URL of the HWaaS API.
      '';
      example = "https://api.hwaas.placeholder.com/v5";
    };

    machines = mkOption {
      type = attrsOf (submodule { options = machineOptions; });
      description = ''
        A set of machine configurations.
      '';
    };

    networks = mkOption {
      type = attrsOf (listOf (submodule { options = connectionOptions; }));
      description = ''
        A set of virtual networks consisting of a list of connections.
      '';
    };

    testScript = mkOption {
      type = either str (functionTo str);
      description = ''
        The actual test that is passed to the NixOS integration test.
      '';
    };
  };
}
