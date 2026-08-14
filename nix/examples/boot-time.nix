# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ pkgs, hwaasTestModules }:
let
  image = (pkgs.nixos hwaasTestModules.user-tooling-machines-legacyBox).isoImage;
  controlVMIp = "192.168.44.1";
  controlVMInterface = "network1";
in
pkgs.hwaasTest {
  name = "Boot-time benchmark";

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
      platform = "legacy-box";
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

  extraPythonPackages = _: [ pkgs.hwaasTimer ];

  testScript = _: ''
    from hwaas_timer import Timer

    start_all()
    controlVM.wait_for_unit("default.target")

    def ping_fn() -> bool:
      status, msg = controlVM.execute("ping -c 1 192.168.44.2")
      return (status == 0)

    # Instruct the timer to run until we can ping the HWaaS machine,
    # but no longer than 10 minutes. Please refer to the documentation
    # of the hwaas_timer for more information.
    timer = Timer()
    timer.set_wait_until_success(ping_fn, 0.5)
    timer.set_timeout(600)

    timer.start()
    hwaas.machines["legacy-box"].power_on()
    timer.wait()

    assert not timer.is_timeout_expired(), "Could not ping HWAAS machine"

    # The result of this benchmark is the time in seconds it took from
    # starting the HWaaS machine until we were able ping it.

    # ! Warning: upload to ElasticSearch is currently omitted in this example !
    # data_collector = get_collector()
    # data_collector.add_metadata({ "name" : "boot-time" })
    # data_collector.add_result({ "seconds" : timer.duration() })
  '';
}
