# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ config, lib, ... }:
let
  wsNetCfg = config.services.websocketProxyGatewayNetwork;
in
{
  options.services.websocketProxyGatewayNetwork = {
    enable = lib.mkEnableOption "the Websocket Proxy Network setup";

    amount = lib.mkOption {
      type = lib.types.ints.u16;
      default = 256;
      example = 4094;
      description = "Number of VLAN subinterfaces to create";
    };

    baseInterface = lib.mkOption {
      type = lib.types.str;
      example = "eth1";
      description = "The linux network base interface to create the VLAN subinterfaces for";
    };

    namePrefix = lib.mkOption {
      type = lib.types.str;
      default = "wsn";
      example = "vlan";
      description = "The linux network base interface to create the VLAN subinterfaces for";
    };

    networkFilePrefix = lib.mkOption {
      type = lib.types.str;
      default = "40-";
      description = ''
        If a network is managed via systemd, the first file in lexicographic order configures the interface.
        This option allows to alter this prefix.
      '';
    };
  };

  config =
    let
      vlanHelper = import ../lib/vlan-config.nix {
        inherit lib;
        inherit (wsNetCfg) networkFilePrefix;
        prefix = wsNetCfg.namePrefix;
      };
      vc = vlanHelper.vlanConfig {
        inherit (wsNetCfg) baseInterface;
        numberOfVlans = wsNetCfg.amount;
      };
    in
    lib.mkIf wsNetCfg.enable {

      # add vlan configuration
      networking.vlans = vc;

      # Enable promiscuous mode for VLAN subinterface
      systemd.network.networks = vlanHelper.promiscConfig { interfaces = builtins.attrNames vc; };

      # needed for promiscuous setting via systemd
      networking.useNetworkd = true;
    };
}
