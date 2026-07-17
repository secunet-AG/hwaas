# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ pkgs, hwaasTestModules }:
let
  testBase = ip1: ip2: {
    apiUrl = "http://api.hwaas.placeholder.com/v5";

    extraPythonPackages = _: [ pkgs.hwaasTimer ];

    machines = {
      legacy-box_1 = {
        image = "${image}/iso/hwaas.iso";
        platform = "legacy-box";
      };
      legacy-box_2 = {
        image = "${image}/iso/hwaas.iso";
        platform = "legacy-box";
      };
    };

    testScript = { ... }: ''
      from hwaas_timer import Timer

      start_all()
      controlVM.wait_for_unit("default.target")

      def ping_fn() -> bool:
        status1, _ = controlVM.execute("ping -c 1 ${ip1}")
        status2, _ = controlVM.execute("ping -c 1 ${ip2}")
        return ((status1 + status2) == 0)

      timer = Timer()
      timer.set_wait_until_success(ping_fn, 0.5)
      timer.set_timeout(600)

      timer.start()
      hwaas.machines["legacy-box_1"].power_on()
      hwaas.machines["legacy-box_2"].power_on()
      timer.wait()

      def maybe_print_cmd(cmd: str) -> None:
        status, msg = controlVM.execute(cmd)
        if status == 0:
          print(f"Output of {cmd}:")
          print(msg)
          print()

      maybe_print_cmd("networkctl status network1")
      maybe_print_cmd("networkctl status network2")

      assert not timer.is_timeout_expired(), "Could not ping legacy-box_1"
    '';
  };

  image = (pkgs.nixos hwaasTestModules.machines.legacyBox).isoImage;
in
{
  singleNetwork = pkgs.hwaasTest
    ({
      name = "Multiple HWaaS machines in one network";

      nodes.controlVM = { ... }: {
        imports = [ hwaasTestModules.hwaasTestVm ];

        hwaas.testVm = {
          enable = true;
          networks = {
            network1 = {
              ipv4Address = { address = "192.168.44.1"; prefixLength = 24; };
              dhcp = true;
              dhcpConfig = {
                ServerAddress = "192.168.44.1/24";
                PoolSize = 3;
              };
            };
          };
        };
      };

      networks = {
        network1 = [
          { machine = "legacy-box_1"; networkInterfaces = [ "LAN1" ]; }
          { machine = "legacy-box_2"; networkInterfaces = [ "LAN1" ]; }
        ];
      };
    } // (testBase "192.168.44.2" "192.168.44.3"));

  multiNetwork = pkgs.hwaasTest
    ({
      name = "Multiple HWaaS machines in separated networks";

      nodes.controlVM = { ... }: {
        imports = [ hwaasTestModules.hwaasTestVm ];

        hwaas.testVm = {
          enable = true;
          networks = {
            network1 = {
              ipv4Address = { address = "192.168.44.1"; prefixLength = 24; };
              dhcp = true;
            };
            network2 = {
              ipv4Address = { address = "192.168.45.1"; prefixLength = 24; };
              dhcp = true;
            };
          };
        };
      };

      networks = {
        network1 = [
          { machine = "legacy-box_1"; networkInterfaces = [ "LAN1" ]; }
        ];
        network2 = [
          { machine = "legacy-box_2"; networkInterfaces = [ "LAN1" ]; }
        ];
      };
    } // (testBase "192.168.44.2" "192.168.45.2"));
}
