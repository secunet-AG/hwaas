# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ lib, ... }:
let
  inherit (builtins) listToAttrs map mapAttrs;

  # The input looks like the following:
  # {
  #   machine = "machine1";
  #   networkInterfaces = [ "LAN1" "LAN2" ];
  # }
  # We want a conversion to the following scheme:
  # {
  #   name = "machine1";
  #   value = {
  #     LAN1 = {};
  #     LAN2 = {};
  #   };
  # }
  mapMachineToNameValuePair = { machine, networkInterfaces }: {
    name = machine;
    value = lib.genAttrs networkInterfaces (_dev: { });
  };

  # The input looks like the following:
  # [
  #   { machine = "machine1"; networkInterfaces = [ "LAN1" "LAN2" ]; }
  #   { machine = "machine2"; networkInterfaces = [ "LAN1" ]; }
  # ]
  # We want a conversion to the following scheme:
  # {
  #   machine1 = {
  #     LAN1 = {};
  #     LAN2 = {};
  #   };
  #   machine2 = {
  #     LAN1 = {};
  #   };
  # }
  mapMachines = machines: listToAttrs (map mapMachineToNameValuePair machines);

  # To get a elasticsearch friendly meta information of networks we want
  # to convert the network items of the hwaas test driver to a convenient form.
  # The input looks like the following:
  # {
  #   network1 = [
  #     { machine = "machine1"; networkInterfaces = [ "LAN1" "LAN2" ]; }
  #     { machine = "machine2"; networkInterfaces = [ "LAN1" ]; }
  #   ];
  # }
  # We want a conversion to the following scheme:
  # {
  #   network1 = {
  #     machine1 = {
  #       LAN1 = {};
  #       LAN2 = {};
  #     };
  #     machine2 = {
  #       LAN1 = {};
  #     };
  #   };
  # }
  mapNetworks = mapAttrs (_name: mapMachines);
in
{
  inherit mapMachineToNameValuePair mapMachines mapNetworks;
}
