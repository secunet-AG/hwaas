# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ nixosTest
, debugging ? false
, modules
,
}:
let
  # NixOS module shared between all VMs
  sharedModule =
    { pkgs, ... }:
    {
      environment.systemPackages = [ pkgs.iperf ];
      networking = {
        firewall.enable = false;
        dhcpcd.enable = false;
        useDHCP = false;
      };
      systemd.network.wait-online.enable = false;
      networking.enableIPv6 = false;
    };

  # The following contains all constanst needed by this scenario
  # server used IPs
  serverIp = "11.11.11.1";
  serverPort = 8080;

  # name of the client TAPs
  clientWsTap = "tapWS";

  # Testnetworks
  testNet1 = {
    vlan = 42;
    hostnet = 3;
    ipPrefix = "192.168.1.";
  };
  testNet2 = {
    vlan = 60;
    hostnet = 4;
    ipPrefix = "192.168.2.";
  };
  testNet3 = {
    vlan = 14;
    hostnet = 5;
    ipPrefix = "192.168.3.";
  };

  # Configure the SUTs (IPs should below x.x.x.20 as clients start at .22)
  sut1Conf = {
    net = testNet1;
    ip = testNet1.ipPrefix + "5";
  };
  sut2Conf = {
    net = testNet1;
    ip = testNet1.ipPrefix + "6";
  };
  sut3Conf = {
    net = testNet2;
    ip = testNet2.ipPrefix + "5";
  };
  sut4Conf = {
    net = testNet3;
    ip = testNet3.ipPrefix + "5";
  };

  sutTemplate = import ./nodes/sut-template.nix {
    inherit sharedModule debugging;
    inherit (modules) test-debug-module;
  };

  # Helper to keep client definition small
  clientOnSutTemplate =
    idx: conf:
    { ... }:
    {
      imports = [
        sharedModule
        ./nodes/client-template.nix
        modules.test-debug-module
        modules.ws-client-module
      ];

      services.simHwaasClient = {
        enable = true;
        inherit serverIp serverPort clientWsTap;
        net = conf.net.vlan;
        ip = "11.11.11.${builtins.toString (idx + 1)}";
        ipTap = conf.net.ipPrefix + "${builtins.toString (idx + 22)}";
        sutIp = conf.ip;
      };
    };

in
nixosTest {
  name = "clients-and-suts-websocket-proxy-scenario";

  # NixOS tests are run inside a virtual machine, and here we specify system of the machine.
  nodes = {

    # The clients simulates developers systems interacting in parallel with a SUT .
    # In this scenario the clients are connected directily to the server - in real life
    # this could be any routed network (e.g. internet).
    # The websocket-proxy-client is runing on each of this clients and
    # they connect to the counterpart the websocket-proxy-gateway runing on the server (aka. gateway).
    # On success there will be an tap interface (like tapWS) on the machine which is usable for communicating with the
    # static HWaaS like network.
    client1 = clientOnSutTemplate 1 sut1Conf;
    client2 = clientOnSutTemplate 2 sut2Conf;
    client3 = clientOnSutTemplate 3 sut3Conf;
    client4 = clientOnSutTemplate 4 sut4Conf;

    # The server node poses the HWaaS Gateway. Once the client connects to the websocketProxyGateway
    # its traffic is tagged with a VLAN and forwarded to the switch.
    # In order to enable the websocketProxyGateway the corresponding VLAN subinterface must be in
    # promiscuous mode. This is ensured via systemd-networkd configuration
    gateway = import ./nodes/gateway.nix {
      inherit
        debugging
        sharedModule
        serverIp
        serverPort
        modules
        ;
    };

    # The switch node simply emulates a test-network switch (later this is a piece of hardware)
    # Traffic comming in from a "tagged" VLAN port/trunk link (eth1)
    # is forwarded to corresponding "untagged" ports (eth2).
    switch =
      { lib
      , ...
      }:
      {
        # This is/are the network(s) not the vlan :)
        virtualisation.vlans = [
          2
          testNet1.hostnet
          testNet2.hostnet
          testNet3.hostnet
        ];

        imports = [
          sharedModule
          modules.test-debug-module
        ];

        services.debugging.enable = debugging;

        # configure the interfaces
        networking = {
          enableIPv6 = false;
          interfaces = {
            eth1.ipv4.addresses = lib.mkForce [ ];
            eth1.ipv6.addresses = lib.mkForce [ ];
            eth2.ipv4.addresses = lib.mkForce [ ];
            eth2.ipv6.addresses = lib.mkForce [ ];
            eth3.ipv4.addresses = lib.mkForce [ ];
            eth3.ipv6.addresses = lib.mkForce [ ];
            eth4.ipv4.addresses = lib.mkForce [ ];
            eth4.ipv6.addresses = lib.mkForce [ ];
          };
          vlans = {
            vlanNet1 = {
              id = testNet1.vlan;
              interface = "eth1";
            };
          };
          vlans = {
            vlanNet2 = {
              id = testNet2.vlan;
              interface = "eth1";
            };
          };
          vlans = {
            vlanNet3 = {
              id = testNet3.vlan;
              interface = "eth1";
            };
          };
          bridges = {
            "br0".interfaces = [
              "vlanNet1"
              "eth2"
            ];
            "br1".interfaces = [
              "vlanNet2"
              "eth3"
            ];
            "br2".interfaces = [
              "vlanNet3"
              "eth4"
            ];
          };
        };
      };

    # This node pose a SUT connected to the switch.
    # For this test, the SUT runs an arbitrary service that the client wants to access.
    # In this particular case the service is a iperf3 server.
    # During the test the client node will pose the iperf3 counterpart.
    sut1 = sutTemplate {
      inherit (sut1Conf.net) hostnet;
      inherit (sut1Conf) ip;
    };
    sut2 = sutTemplate {
      inherit (sut2Conf.net) hostnet;
      inherit (sut2Conf) ip;
    };
    sut3 = sutTemplate {
      inherit (sut3Conf.net) hostnet;
      inherit (sut3Conf) ip;
    };
    sut4 = sutTemplate {
      inherit (sut4Conf.net) hostnet;
      inherit (sut4Conf) ip;
    };
  };

  # Disable linting for simpler debugging of the testScript
  # skipLint = debugging;

  testScript = ''
    from typing import List

    # Start the fake HWaaS
    switch.start()
    gateway.start()
    gateway.wait_for_unit("websocket-proxy-gateway.service")
    gateway.wait_for_open_port(${builtins.toString serverPort})

    # spawn SUTs and the clients (wait until they are connected)
    start_all()
    all_clients: List[Machine] = [client1, client2, client3, client4]
    for c in all_clients:
      c.wait_for_unit("websocket-proxy-client.service")
      c.wait_for_unit("default.target")

    sut1.wait_for_unit("default.target")

    with subtest("Clients ping their SUT"):
      # Start a ping on all clinets thowards their configured SUT
      for c in all_clients:
        c.succeed("ping -I ${clientWsTap} -c 10 sut")

    with subtest("Client could not ping a SUT on another network"):
      client1.fail("ping -I ${clientWsTap} -c 1 ${sut4Conf.ip}")

  '';
}
