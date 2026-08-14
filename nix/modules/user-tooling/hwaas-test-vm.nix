# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

wsProxyClient:
{ config, lib, ... }:
let
  testVmCfg = config.hwaas.testVm;

  networkIpv4Address = {
    address = lib.mkOption {
      type = lib.types.str;
      description = "The IPv4 Address of the network interface.";
    };

    prefixLength = lib.mkOption {
      type = lib.types.ints.between 0 32;
      description = "The network prefix length of the ipv4 Address.";
    };
  };

  networkOptions = lib.types.submodule {
    options = {
      ipv4Address = lib.mkOption {
        type = lib.types.submodule { options = networkIpv4Address; };
        description = "The IPv4 address configuration of the interface, including network prefix length.";
      };

      dhcp = lib.mkEnableOption "DHCP on this interface";

      dhcpConfig = lib.mkOption {
        type = with lib.types; nullOr anything;
        default = null;
        description = ''
          A configuration for the DHCP Server on this interface. Will be passed to
                    `systemd.network.networks.<name>.dhcpServerConfig`. If this is left empty, a default
                    configuration that hands out one IP address will be used.'';
        example = {
          ServerAddress = "192.168.44.1/24";
          PoolSize = 3;
        };
      };
    };
  };
in
{
  imports = [ wsProxyClient ];

  options.hwaas.testVm = {
    enable = lib.mkEnableOption ''
      Enable control VM functionality (e.g. HWaaS network connection via
      websocket).
    '';

    sharedDirectory = lib.mkOption {
      type = with lib.types; path;
      default = "/var/lib/hwaas";
      description = ''
        Directory created in the testVm that contains information passed from
        the HWaaS python driver to the testVm e.g. the websocket URL to a
        HWaaS network.
      '';
    };
    networks = lib.mkOption {
      type = lib.types.attrsOf networkOptions;
      default = {
        tap0 = {
          ipv4Address = {
            address = "192.168.44.1";
            prefixLength = 24;
          };
        };
      };
      description = "The network configuration of the tap device(s) used to connect to the HWaaS Network";
    };
  };

  config = lib.mkIf testVmCfg.enable {
    networking = {
      useDHCP = lib.mkForce false;
      networkmanager.enable = lib.mkForce false;
      useNetworkd = lib.mkForce true;
      # We open the ports needed for DHCP by default.
      firewall.allowedUDPPorts = [
        67
        68
      ];
    };

    services.websocketProxyClient = {
      enable = true;
      networks = builtins.mapAttrs (iface: _: {
        envFile = "${testVmCfg.sharedDirectory}/network.conf.${iface}";
      }) testVmCfg.networks;
    };

    systemd.mounts = [
      {
        what = "hwaasDir";
        where = builtins.toString testVmCfg.sharedDirectory;
        type = "9p";
        wantedBy = [
          "websocket-proxy-client.service"
          "network.target"
        ];
        enable = true;
      }
    ];

    virtualisation.qemu.options = [
      "-virtfs local,path=\$HWAASDIR,mount_tag=hwaasDir,security_model=mapped-xattr,id=hwaasDir"
    ];

    systemd.network = {
      enable = true;
      netdevs = builtins.mapAttrs (iface: _: {
        netdevConfig = {
          Kind = "tap";
          Name = iface;
        };
      }) testVmCfg.networks;

      networks = {
        "50-ethernet" = {
          enable = true;
          matchConfig.Type = "ether";
          DHCP = "ipv4";
          linkConfig.RequiredForOnline = "no";
        };
      }
      // lib.mapAttrs' (
        iface: net:
        lib.nameValuePair "20-${iface}" (
          {
            enable = true;
            matchConfig.Name = iface;
            address = with net.ipv4Address; [ "${address}/${builtins.toString prefixLength}" ];
            linkConfig.RequiredForOnline = "no";
            networkConfig.ConfigureWithoutCarrier = "yes";
          }
          // lib.optionalAttrs net.dhcp {
            networkConfig.DHCPServer = true;
            dhcpServerConfig =
              if net.dhcpConfig != null then
                net.dhcpConfig
              else
                {
                  ServerAddress = with net.ipv4Address; "${address}/${builtins.toString prefixLength}";
                  PoolSize = 2;
                };
          }
        )
      ) testVmCfg.networks;
    };

  };
}
