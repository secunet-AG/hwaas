# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{
  nixosTest,
  lib,
  debugging ? false,
  modules,
  ws-proxy-client,
}:
let
  # NixOS module shared between all VMs
  sharedModule = { pkgs, ... }: {
    environment.systemPackages = with pkgs; [ iperf ];
    networking = {
      firewall.enable = false;
      dhcpcd.enable = false;
      useDHCP = false;
    };
  };

  # const
  clientIpTap = "192.168.1.22";
  clientIp = "11.11.11.2";
  serverIp = "11.11.11.1";
  sutIp = "192.168.1.5";
  serverPort = 8080;
  clientWsTap = "tapWS";
  staticVlan = 42;

  # Construct a large VLAN config for gateway
  inherit (import ../../lib/vlan-config.nix { inherit lib; }) vlanConfig vlanIfaceName;

in
nixosTest {
  name = "static-websocket-proxy-scenario";

  # NixOS tests are run inside a virtual machine, and here we specify system of the machine.
  nodes = {

    # The client simulates a developers machine.
    # In this scenario it is connected directily to the server - in real life this could be any network (e.g. internet).
    # The websocketProxyClient is running on this node and
    # connects to the counterpart running on the server (aka. gateway).
    # On success there will be a tap interface (like tapWS) on the machine which is usable for communicating with the
    # static HWaaS like network.
    client = { lib, ... }: {
      # This is/are the network(s) not the vlan :)
      virtualisation.vlans = [ 1 ];

      imports = [
        sharedModule
        modules.test-debug-module
        modules.ws-client-module
      ];

      services.debugging.enable = debugging;

      networking = {
        extraHosts = lib.optionalString debugging "${sutIp} sut";
        interfaces.eth1.ipv4.addresses = lib.mkForce [
          {
            address = clientIp;
            prefixLength = 24;
          }
        ];
        interfaces.${clientWsTap} = {
          ipv4.addresses = [
            {
              address = clientIpTap;
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
        baseInterface = clientWsTap;
        uri = "ws://${serverIp}:${builtins.toString serverPort}/ws/${builtins.toString staticVlan}";
      };

      # The test needs control over starting the unit - prevent autostart
      systemd.services.websocket-proxy-client.wantedBy = lib.mkForce [ ];

      environment.systemPackages = [ ws-proxy-client ];

    };

    # The server node poses the HWaaS Gateway. Once the client connects to the websocketProxyGateway
    # its traffic is tagged with a VLAN and forwarded to the switch.
    # In order to enable the websocketProxyGateway the corresponding VLAN subinterface must be in
    # promiscuous mode. This is ensured via systemd-networkd configuration
    server = _: {
      # This is/are the network(s) not the vlan :)
      virtualisation.vlans = [
        1
        2
      ];

      imports = [
        sharedModule
        modules.test-debug-module
        modules.ws-gateway-module
      ];

      services.debugging.enable = debugging;

      networking = {
        useNetworkd = true;
        interfaces = {
          eth1.ipv4.addresses = lib.mkForce [
            {
              address = serverIp;
              prefixLength = 24;
            }
          ];
          eth2.ipv4.addresses = lib.mkForce [ ];
          eth2.ipv6.addresses = lib.mkForce [ ];
        };
        vlans = vlanConfig {
          baseInterface = "eth2";
          numberOfVlans = 50;
        };
      };

      # Enable promiscuous mode for VLAN subinterface
      systemd.network.networks.${vlanIfaceName staticVlan} = {
        name = "${vlanIfaceName staticVlan}";
        linkConfig = {
          Promiscuous = true;
        };
      };

      # Configure the websocketProxyGateway service
      services.websocketProxyGateway = {
        enable = true;
        openFirewall = true;
        port = serverPort;
      };

    };

    # The switch node simply emulates a test-network switch (later this is a piece of hardware)
    # Traffic comming in from a "tagged" VLAN port/trunk link (eth1)
    # is forwarded to corresponding "untagged" ports (eth2).
    switch = _: {
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

      # configure the interfaces and bridge them
      networking = {
        interfaces = {
          eth1.ipv4.addresses = lib.mkForce [ ];
          eth2.ipv4.addresses = lib.mkForce [ ];
          eth2.ipv6.addresses = lib.mkForce [ ];
        };
        vlans = {
          vlan42 = {
            id = 42;
            interface = "eth1";
          };
        };
        bridges."br0".interfaces = [
          "vlan42"
          "eth2"
        ];
      };
    };

    # This node pose a SUT connected to the switch.
    # For this test, the SUT runs an arbitrary service that the client wants to access.
    # In this particular case the service is a iperf3 server.
    # During the test the client node will pose the iperf3 counterpart.
    sut = { pkgs, ... }: {
      # This is/are the network(s) not the vlan :)
      virtualisation.vlans = [ 3 ];

      imports = [
        sharedModule
        modules.test-debug-module
      ];

      services.debugging.enable = debugging;

      # setup the interface so the client could connect to the service.
      networking = {
        interfaces.eth1.ipv4.addresses = lib.mkForce [
          {
            address = sutIp;
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

    };
  };

  # Disable linting for simpler debugging of the testScript
  # skipLint = debugging;

  testScript = ''
    start_all()

    server.wait_for_unit("websocket-proxy-gateway.service")
    server.wait_for_open_port(${builtins.toString serverPort})

    with subtest("Second attempt must fail"):
      faultyConnect = "ws-proxy-client -vv --address ws://${serverIp}:${builtins.toString serverPort}/ws/${
        builtins.toString (staticVlan + 100)
      } ${clientWsTap}"
      client.fail(faultyConnect, timeout=5)

      rc, out = client.execute(faultyConnect, timeout=5)
      assert rc != 0, "must not succeed"
      assert rc != 124, "must not timeout"


    with subtest("Reconnect possible"):
      client.systemctl("start websocket-proxy-client.service --no-pager")
      client.wait_for_unit("websocket-proxy-client.service")
      client.succeed("ping -I ${clientWsTap} -c 1 ${sutIp}")
      client.systemctl("stop websocket-proxy-client.service --no-pager")
      client.fail("ping -I ${clientWsTap} -c 1 ${sutIp}")
      client.systemctl("start websocket-proxy-client.service --no-pager")
      client.wait_for_unit("websocket-proxy-client.service")
      client.succeed("ping -I ${clientWsTap} -c 1 ${sutIp}")
  '';
}
