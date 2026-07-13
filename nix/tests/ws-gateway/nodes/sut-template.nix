# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ debugging
, sharedModule
, test-debug-module
,
}:
{ hostnet, ip }:
{ pkgs
, lib
, ...
}:
{
  # This is/are the network(s) not the vlan :)
  virtualisation.vlans = [ hostnet ];

  imports = [
    sharedModule
    test-debug-module
  ];

  services.debugging.enable = debugging;

  # setup the interface so the client could connect to the service.
  networking = {
    interfaces.eth1.ipv4.addresses = lib.mkForce [
      {
        address = ip;
        prefixLength = 24;
      }
    ];
  };

  # start the iperf3 server as systemd service on this system.
  systemd.services.iperf3-server = {
    description = "Run the iperf3 server for test scenario";
    wantedBy = [ "default.target" ];
    # wait until network is online
    after = [ "network-online.target" ];
    wants = [ "network-online.target" ];
    serviceConfig = {
      ExecStart = "${pkgs.iperf3}/bin/iperf3 -s -p 7575";
    };
  };

}
