# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{
  debugging,
  sharedModule,
  serverIp,
  serverPort,
  vlanInterfaceCount ? null,
  modules,
}:
{ lib, ... }: {
  # This is/are the network(s) not the vlan :)
  virtualisation.vlans = [
    1
    2
  ];
  # increase number of vCPUs
  virtualisation.cores = 4;

  imports = [
    sharedModule
    modules.test-debug-module
    modules.ws-gateway-module
    modules.ws-gateway-net-module
  ];
  services = {

    debugging.enable = debugging;

    # Configure the websocketProxyGateway service
    websocketProxyGatewayNetwork = {
      enable = true;
      baseInterface = "eth2";
    }
    // lib.optionalAttrs (vlanInterfaceCount != null) { amount = vlanInterfaceCount; };

    # Configure the websocketProxyGateway service
    websocketProxyGateway = {
      enable = true;
      openFirewall = true;
      port = serverPort;
    };
  };

  networking = {
    interfaces.eth1.ipv4.addresses = lib.mkForce [
      {
        address = serverIp;
        prefixLength = 24;
      }
    ];
    interfaces.eth2.ipv4.addresses = lib.mkForce [ ];
  };

}
