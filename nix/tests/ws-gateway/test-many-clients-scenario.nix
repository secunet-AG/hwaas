# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{
  nixosTest,
  debugging ? false,
  modules,
}:
let
  # NixOS module shared between all VMs
  sharedModule = { pkgs, ... }: {
    environment.systemPackages = [ pkgs.iperf ];
    networking = {
      firewall.enable = false;
      dhcpcd.enable = false;
      useDHCP = false;
    };
    systemd.network.wait-online.enable = false;
  };

  # used constants
  serverIp = "11.11.11.1";
  serverPort = 8080;
  sutIp = "192.168.1.5";
  clientWsTap = "tapWS";
  staticVlan = 9;
  vlanName = "vlan${builtins.toString staticVlan}";

  sutTemplate = import ./nodes/sut-template.nix {
    inherit sharedModule debugging;
    inherit (modules) test-debug-module;
  };
  clientsOnSutNet = import ./nodes/clients-on-sut.nix {
    inherit
      serverIp
      serverPort
      sharedModule
      modules
      ;
  };
  # for clientNodes number >= 10 adapt test script as the clientNodeNamesString start with client10..
  number = 9;
  clientNodes = clientsOnSutNet {
    inherit number;
    net = staticVlan;
    inherit sutIp;
  };
  clientNodeNamesString = builtins.concatStringsSep ", " (builtins.attrNames clientNodes);
in
nixosTest {
  name = "many-clients-websocket-proxy-scenario";

  # NixOS tests are run inside a virtual machine, and here we specify system of the machine.
  nodes =

    # The clients simulates developers systems interacting in parallel with a SUT.
    # In this scenario, the clients are connected directly to the server - in real life
    # this could be any routed network (e.g. internet).
    # The websocket-proxy-client is running on each of these clients and
    # they connect to the counterpart the websocket-proxy-gateway running on the server (aka. gateway).
    # On success there will be a tap interface (like tapWS) on the machine which is usable for communicating with the
    # static HWaaS like network.
    clientNodes // {

      # The server node poses the HWaaS Gateway. Once the client connects to the websocketProxyGateway
      # its traffic is tagged with a VLAN and forwarded to the switch.
      # In order to enable the websocketProxyGateway the corresponding VLAN subinterface must be in
      # promiscuous mode. This is ensured via systemd-networkd configuration
      server = import ./nodes/gateway.nix {
        inherit
          debugging
          sharedModule
          serverIp
          serverPort
          modules
          ;
        vlanInterfaceCount = number;
      };

      # The switch node simply emulates a test-network switch (later this is a piece of hardware)
      # Traffic comming in from a "tagged" VLAN port/trunk link (eth1)
      # is forwarded to corresponding "untagged" ports (eth2).
      switch = { lib, ... }: {
        # This is/are the network(s) not the vlan :)
        virtualisation.vlans = [
          2
          3
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
            eth2.ipv4.addresses = lib.mkForce [ ];
            eth2.ipv6.addresses = lib.mkForce [ ];
          };
          vlans = {
            "${vlanName}" = {
              id = staticVlan;
              interface = "eth1";
            };
          };
          bridges."br0".interfaces = [
            vlanName
            "eth2"
          ];
        };
      };

      # This node pose a SUT connected to the switch.
      # For this test, the SUT runs an arbitrary service that the client wants to access.
      # In this particular case the service is a iperf3 server.
      # During the test the client node will pose the iperf3 counterpart.
      sut = sutTemplate {
        hostnet = 3;
        ip = sutIp;
      };
    };

  # Disable linting for simpler debugging of the testScript
  # skipLint = debugging;

  testScript = ''
    from itertools import permutations

    def rotate(l, n):
      return l[n:] + l[:n]

    # Start the fake HWaaS
    switch.start()
    server.start()
    sut.start()
    server.wait_for_unit("websocket-proxy-gateway.service")
    server.wait_for_open_port(${builtins.toString serverPort})

    # spawn the clients and wait until they are connected
    start_all()
    all_clients = [${clientNodeNamesString}]
    for c in all_clients:
      c.wait_for_unit("websocket-proxy-client.service")
      c.wait_for_unit("default.target")

    sut.wait_for_unit("default.target")

    with subtest("Clients can ping SUT"):
      for c in all_clients:
        c.succeed("ping -I ${clientWsTap} -c 1 ${sutIp}")

    with subtest("SUT can ping all clients"):
      for i in range(0,len(all_clients)):
        sut.succeed(f"ping -c 1 192.168.1.{22+i}")

    # Clients must not be able to ping eachother directly via ws-proxy.
    # doing a cyclic ping between clients where none should succeed
    with subtest("Clients could not ping eachother"):
      cyclic_ping_cfg = zip(all_clients, range(0,len(all_clients)))
      for ((client_a, _), (__, idx_b)) in permutations(cyclic_ping_cfg, r=2):
        assert str(idx_b+1) not in client_a.name.split("Net")[0], "ping config missmatch"

        ip_b = f"192.168.1.{22+idx_b}"
        assert ip_b not in client_a.succeed("ip a"), "suspicious IP address config"

        client_a.fail(f"ping -I ${clientWsTap} -c 2 -W .2 {ip_b}")
  '';
}
