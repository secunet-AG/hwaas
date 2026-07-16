# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ pkgs, hwaasTestModules }:
let
  image = (pkgs.nixos hwaasTestModules.machines.legacyBox).isoImage;
  controlVMIp = "192.168.44.1";
  controlVMInterface = "network1";
  platform = "legacy-box";
in
{
  pingHWaaSMachine = pkgs.hwaasTest {
    name = "Ping HWaaS Machine via VLAN Test";

    apiUrl = "http://api.hwaas.placeholder.com/v5";

    nodes.controlVM = { ... }:
      {
        imports = with hwaasTestModules; [
          hwaasTestVm
        ];

        hwaas.testVm = {
          enable = true;
          networks = {
            "${controlVMInterface}" = {
              ipv4Address = { address = controlVMIp; prefixLength = 24; };
              dhcp = true;
            };
          };
        };
      };

    machines = {
      legacy-box = {
        image = "${image}/iso/hwaas.iso";
        inherit platform;
      };
    };

    networks = {
      "${controlVMInterface}" = [
        { machine = "legacy-box"; networkInterfaces = [ "LAN1" ]; }
      ];
    };

    testScript = { ... }: ''
      import time

      start_all()
      controlVM.wait_for_unit("default.target")

      # Calling the `machines` endpoint of the HWaaS API.
      # Equivalent to this call with the `hwaas_connector`:
      # `hwaas.hwaas_connector.get(f"contexts/{hwaas.context}/machines/legacy-box")`
      response = hwaas.machines["legacy-box"].get_machine_info()
      assert response.get("platform") == "${platform}", f"machine platform '${platform}' should be in response '{response}'"
      assert "id" in response, "id-key should be in response dict"
      print(f"The machine id is {response['id']}")

      # Power on machine by using the `power_on` function of the user-tooling
      hwaas.machines["legacy-box"].power_on()

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

      hwaas.machines["legacy-box"].usb_detach()
      hwaas.machines["legacy-box"].power_off()
    '';
  };
}
