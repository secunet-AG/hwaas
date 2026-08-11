# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ config, lib, ... }:
let
  clientCfg = config.services.simHwaasClient;
in
{
  options.services.simHwaasClient = {
    enable = lib.mkEnableOption "a simulated HWaaS client/user";

    debugging = lib.mkEnableOption "add debug tools and configs";

    net = lib.mkOption {
      type = lib.types.int;
      example = 42;
      description = "The HWaaS Network to connect to";
    };

    ip = lib.mkOption {
      type = lib.types.str;
      example = "192.168.1.1";
      description = "The static IP of the client";
    };

    ipTap = lib.mkOption {
      type = lib.types.str;
      example = "192.168.1.1";
      description = "The static IP of the TAP interface for HWaaS communicatuion";
    };

    sutIp = lib.mkOption {
      type = lib.types.str;
      example = "192.168.1.1";
      description = "The IP of the SUT to add to the hosts config";
    };

    serverIp = lib.mkOption {
      type = lib.types.str;
      example = "192.168.1.1";
      description = "The IP of the HWaaS (websocket-proxy-) gateway to connect to";
    };
    serverPort = lib.mkOption {
      type = lib.types.port;
      example = 8080;
      description = "The Port of the HWaaS (websocket-proxy-) gateway to connect to";
    };

    clientWsTap = lib.mkOption {
      type = lib.types.str;
      default = "tapWS";
      example = "tap0";
      description = "suggested name of the tap interface to spawn";
    };
  };

  config = lib.mkIf clientCfg.enable {
    # This is/are the network(s) not the vlan :)
    virtualisation.vlans = [ 1 ];

    services.debugging.enable = clientCfg.debugging;

    networking = {
      extraHosts = "${clientCfg.sutIp} sut";
      interfaces.eth1.ipv4.addresses = lib.mkForce [
        {
          address = clientCfg.ip;
          prefixLength = 24;
        }
      ];
      interfaces.${clientCfg.clientWsTap} = {
        ipv4.addresses = [
          {
            address = clientCfg.ipTap;
            prefixLength = 24;
          }
        ];
        virtualType = "tap";
        virtual = true;
        mtu = 1470;
      };
    };

    services.websocketProxyClient = {
      enable = true;
      baseInterface = clientCfg.clientWsTap;
      uri = "ws://${clientCfg.serverIp}:${builtins.toString clientCfg.serverPort}/ws/${builtins.toString clientCfg.net}";
    };

  };

}
