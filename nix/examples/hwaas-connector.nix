# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ pkgs, hwaasTestModules }:
let
  image = (pkgs.nixos hwaasTestModules.user-tooling-machines-legacyBox).isoImage;
  controlVMIp = "192.168.44.1";
  controlVMInterface = "network1";
in
{
  hwaasConnector = pkgs.hwaasTest {
    name = "Test HWaaS Connector";

    apiUrl = "http://api.hwaas.placeholder.com/v5";

    nodes.controlVM = { ... }:
      {
        imports = with hwaasTestModules; [
          user-tooling-hwaasTestVm
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
        platform = "legacy-box";
      };
    };

    networks = {
      "${controlVMInterface}" = [
        { machine = "legacy-box"; networkInterfaces = [ "LAN1" ]; }
      ];
    };

    testScript = _: ''
      import time
      import json

      start_all()
      controlVM.wait_for_unit("default.target")

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

      def get_machines():
        response = hwaas.hwaas_connector.get(f"contexts/{hwaas.context}/machines")
        assert response.status_code == 200
        assert "legacy-box" in json.loads(response.text)

      get_machines()
    '';
  };
}
