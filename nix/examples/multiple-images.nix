# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ pkgs, hwaasTestModules }:
let
  testBase = image_num: ip: {
    apiUrl = "http://api.hwaas.placeholder.com/v5";

    nodes.controlVM = { ... }: {
      imports = with hwaasTestModules; [ user-tooling-hwaasTestVm ];
      environment.systemPackages = with pkgs; [ sshpass ];

      hwaas.testVm = {
        enable = true;
        networks = {
          "network1" = {
            ipv4Address = {
              address = "192.168.44.1";
              prefixLength = 24;
            };
            dhcp = true;
          };
        };
      };
    };
    networks = {
      network1 = [
        {
          machine = "legacy-box_1";
          networkInterfaces = [ "LAN1" ];
        }
      ];
    };
    testScript = _: ''
      import time

      def wait_for_network():
        success = False
        for _ in range(240):
          status, msg = controlVM.execute("ping -c 1 ${ip}")
          print(msg)
          if status == 0:
            success = True
            break
          else:
            time.sleep(1)
        assert success == True, "Could not ping HWAAS machine"

      def image_test():
        image_name = machine.succeed("sshpass -p 1234 ssh -o StrictHostKeyChecking=no nixos@${ip} 'cat /etc/image'")
        assert (str(image_name) == "image${image_num}"), "Other image expected"

        second_image = machine.succeed("sshpass -p 1234 ssh -o StrictHostKeyChecking=no nixos@${ip} 'test -e /dev/sdc; echo $?'")
        assert (int(second_image) == 0), "Additional Image not present"

      start_all()
      controlVM.wait_for_unit("default.target")

      hwaas.machines["legacy-box_1"].power_on()
      wait_for_network()

      image_test()
    '';
  };

  image1 =
    (pkgs.nixos (
      { ... }: {
        imports = [ hwaasTestModules.user-tooling-machines-legacyBox ];
        # Allow ssh access to the HWaaS machine and add the benchmark package
        users.users.nixos = {
          initialHashedPassword = pkgs.lib.mkForce null;
          initialPassword = "1234";
        };
        environment.etc.image = {
          enable = true;
          text = "image1";
        };
      }
    )).isoImage;

  image2 =
    (pkgs.nixos (
      { ... }: {
        imports = [ hwaasTestModules.user-tooling-machines-legacyBox ];
        # Allow ssh access to the HWaaS machine and add the benchmark package
        users.users.nixos = {
          initialHashedPassword = pkgs.lib.mkForce null;
          initialPassword = "1234";
        };
        environment.etc.image = {
          enable = true;
          text = "image2";
        };
      }
    )).isoImage;
in
{
  # Test whether image1 is booted and image2 is integrated as an additional image.
  # Split the test in two parts for stability reasons.
  mounting = pkgs.hwaasTest (
    {
      name = "First Part of Multiple Images";
      machines = {
        legacy-box_1 = {
          image = "${image1}/iso/hwaas.iso";
          platform = "legacy-box";
          additionalImages = [ "${image2}/iso/hwaas.iso" ];
        };
      };
    }
    // (testBase "1" "192.168.44.2")
  );
  # Test whether image2 is booted and the boot sequence is therefore not dependent on other factors, such as the name of the image.
  bootOrder = pkgs.hwaasTest (
    {
      name = "Second Part of Multiple Images";
      machines = {
        legacy-box_1 = {
          image = "${image2}/iso/hwaas.iso";
          platform = "legacy-box";
          additionalImages = [ "${image1}/iso/hwaas.iso" ];
        };
      };
    }
    // (testBase "2" "192.168.44.2")
  );
}
