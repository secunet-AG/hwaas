# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{
  nixosTest,
  lib,
  debugging ? false,
  modules,
}:
let
  # NixOS module shared between server and switch
  sharedModule = _: {
    virtualisation.vlans = [ 1 ];
    networking.firewall.enable = true;
    networking.dhcpcd.enable = false;
  };

  # const
  baseInterface = "eth1";
  vlanInterface = vlanIfaceName 42;
  numberOfVlans = 500;
  serverIp = "10.10.10.1";
  serverIpVlan = "10.1.1.1";
  switchIp = "10.10.10.2";
  switchIpVlan = "10.1.1.2";

  # Construct a large VLAN config
  inherit (import ../../lib/vlan-config.nix { inherit lib; }) vlanConfig vlanIfaceName;

  # define VLANs for server; the last parameter is the amount;
  serverVlans = vlanConfig { inherit baseInterface numberOfVlans; };

  # define VLANs for switch
  switchVlans = {
    wsn42 = {
      id = 42;
      interface = baseInterface;
    };
  };

in
nixosTest {
  name = "vlan-test";

  # NixOS tests are run inside a virtual machine, and here we specify system of the machine.
  nodes = {

    # The server offers many VLAN interfaces
    # In order to allow a ping, an IP address must be configured
    server = { ... }: {
      imports = [
        sharedModule
        modules.test-debug-module
      ];

      services.debugging.enable = debugging;

      networking = {
        interfaces.${baseInterface}.ipv4.addresses = [
          {
            address = serverIp;
            prefixLength = 24;
          }
        ];
        interfaces.${vlanInterface}.ipv4.addresses = [
          {
            address = serverIpVlan;
            prefixLength = 24;
          }
        ];
        vlans = serverVlans;
      };
    };

    # The switch in this scenario only configures one VLAN interface
    # In order to allow a ping, an IP address must be configured
    switch = { ... }: {
      imports = [
        sharedModule
        modules.test-debug-module
      ];
      services.debugging.enable = debugging;

      networking = {
        interfaces.${baseInterface}.ipv4.addresses = [
          {
            address = switchIp;
            prefixLength = 24;
          }
        ];
        interfaces.${vlanInterface}.ipv4.addresses = [
          {
            address = switchIpVlan;
            prefixLength = 24;
          }
        ];
        vlans = switchVlans;
      };
    };
  };

  # Disable linting for simpler debugging of the testScript
  # skipLint = debugging;

  testScript = ''
    start_all()
    server.wait_for_unit("default.target")
    switch.wait_for_unit("default.target")

    PING_RETRY = 1

    pings = [
      # Ping switch.eth1 -> server.eth1
      (switch, "${baseInterface}", "${serverIp}"),

      # Ping switch.vlan42 -> server.vlan42
      (switch, "${vlanInterface}", "${serverIpVlan}"),

      # Ping server.eth1 -> switch.eth1
      (server, "${baseInterface}", "${switchIp}"),

      # Ping switch.vlan42 -> server.vlan42
      (server, "${vlanInterface}", "${switchIpVlan}"),
    ]

    for node, iface, ip in pings:
      cmd = f"ping -I {iface} -c {PING_RETRY} {ip}"
      print(f"Running on {node}: {cmd}")
      node.succeed(cmd)

  '';
}
