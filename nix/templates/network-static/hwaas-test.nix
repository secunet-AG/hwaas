# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
# SPDX-License-Identifier: MIT-0

# This attribute set defines a HWaaS test. It may contain all the following options/attributes:
#
# - NixOS test attributes, see <https://nixos.org/manual/nixos/unstable/#sec-test-options-reference>
#
# The `testScript` can use the following Python APIs:
#
# - NixOS Python methods, see <https://nixos.org/manual/nixos/unstable/#ssec-machine-objects>
{ user-tooling-hwaasTestVm, pkgs }:
let
  # === START OF USER CONFIGURATION ===
  user-config = {
    # NOTE: You MUST adapt these settings to your HWaaS environment
    api-url = "https://api.hwaas.changeme.example/v5";
    bmr-interface-name = "LAN1";
    bmr-platform = "test-machine-x86_64";
    # NOTE: You MAY change these settings to whatever you wish
    bmr-address = "192.168.100.2";
    bmr-name = "dummy1";
    gateway-address = "192.168.100.1";
    gateway-address-prefix = 24;
    gateway-network-name = "myTestNetwork";
  };
  # === END OF USER CONFIGURATION ===

  # Disk image booted on the BMR
  disk_image =
    (pkgs.nixos (
      { modulesPath, lib, ... }: {
        imports = [ "${modulesPath}/installer/cd-dvd/installation-cd-minimal.nix" ];

        image.baseName = lib.mkForce "hwaas";
        networking = {
          useDHCP = lib.mkForce false;
          networkmanager.enable = lib.mkForce false;
          useNetworkd = lib.mkForce true;
          firewall.enable = lib.mkForce false;
          wireless.enable = lib.mkForce false;
          enableIPv6 = false;
        };

        systemd.network = {
          enable = true;
          netdevs.virbr0.netdevConfig = {
            Kind = "bridge";
            Name = "virbr0";
          };
          networks."30-virbr0" = {
            matchConfig.Name = "virbr0";
            addresses = [
              {
                addressConfig.Address = "${user-config.bmr-address}/${builtins.toString user-config.gateway-address-prefix}";
              }
            ];
          };
          # Instead of hard-coding matching network interface names that are
          # different per machine we match with a generic ether type networkd rule
          networks."20-lan-ports" = {
            matchConfig.Type = "ether";
            networkConfig.Bridge = "virbr0";
            linkConfig.RequiredForOnline = "enslaved";
          };
        };
      }
    )).isoImage;
in
{
  name = "HWaaS test with local network via static configuration";

  # === START OF SPECIAL HWAAS TEST ATTRIBUTES ===
  apiUrl = user-config.api-url;
  machines = {
    "${user-config.bmr-name}" = {
      platform = user-config.bmr-platform;
      image = "${disk_image}/iso/hwaas.iso";
    };
  };
  networks = {
    "undefined" = [ ];
  };

  nodes.controlVM = _: {
    imports = [ user-tooling-hwaasTestVm ];

    hwaas.testVm.enable = true;
    hwaas.testVm.networks."${user-config.gateway-network-name}" = {
      ipv4Address = {
        address = user-config.gateway-address;
        prefixLength = user-config.gateway-address-prefix;
      };
    };
  };
  networks."${user-config.gateway-network-name}" = [
    {
      machine = user-config.bmr-name;
      networkInterfaces = [ user-config.bmr-interface-name ];
    }
  ];
  # === END OF SPECIAL HWAAS TEST ATTRIBUTES ===

  testScript = ''
    controlVM.start()
    controlVM.wait_for_unit("multi-user.target")

    hwaas.machines["${user-config.bmr-name}"].power_on()
    print("Machine powered on!")

    controlVM.wait_until_succeeds("ping -c 1 ${user-config.bmr-address}", timeout=900)
    print("Machine responds to ping!")

    hwaas.machines["${user-config.bmr-name}"].power_off()
    print("Machine powered off!")
  '';
}
