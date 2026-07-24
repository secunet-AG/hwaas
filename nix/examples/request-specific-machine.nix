# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ pkgs, hwaasTestModules }:
let
  image = (pkgs.nixos hwaasTestModules.user-tooling-machines-legacyBox).isoImage;
  controlVMIp = "192.168.44.1";
  controlVMInterface = "network1";
  platform = "legacy-box";
  machine_id = 3;
in
pkgs.hwaasTest {
  name = "Tests if a specific machine can be requested by id for a test";

  apiUrl = "http://api.hwaas.placeholder.com/v5";

  nodes.controlVM = { ... }: {
    imports = with hwaasTestModules; [ user-tooling-hwaasTestVm ];

    hwaas.testVm = {
      enable = true;
      networks = {
        "${controlVMInterface}" = {
          ipv4Address = {
            address = controlVMIp;
            prefixLength = 24;
          };
          dhcp = true;
        };
      };
    };
  };

  machines = {
    legacy-box = {
      image = "${image}/iso/hwaas.iso";
      inherit platform machine_id;
    };
  };

  networks = {
    "${controlVMInterface}" = [
      {
        machine = "legacy-box";
        networkInterfaces = [ "LAN1" ];
      }
    ];
  };

  testScript = _: ''
    import time

    start_all()
    controlVM.wait_for_unit("default.target")

    response = hwaas.machines["legacy-box"].get_machine_info()
    assert response.get("platform") == "${platform}", f"machine platform '${platform}' should be in response '{response}'"
    assert response.get("id") == ${toString machine_id}, f"machine id '${toString machine_id}' should be in response '{response}'"

    hwaas.machines["legacy-box"].power_on()

    def wait_for_network():
      success = False

      for _ in range(120):
        status, msg = controlVM.execute("ping -c 1 192.168.44.2")
        print(msg)
        if status == 0:
          success = True
          break
        else:
          time.sleep(1)
      assert success == True, "Could not ping HWAAS machine"

    wait_for_network()
  '';
}
