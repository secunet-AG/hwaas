# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ pkgs, hwaasTestModules }:
let
  image = (pkgs.nixos ({ ... }: {
    imports = [
      hwaasTestModules.user-tooling-machines-legacyBox
    ];

    # Allow ssh access to the HWaaS machine and add the benchmark package
    users.users.nixos = {
      initialHashedPassword = pkgs.lib.mkForce null;
      initialPassword = "1234";
      packages = with pkgs; [ fio ];
    };

  })).isoImage;

  controlVMIp = "192.168.44.1";
  controlVMInterface = "network1";
in
pkgs.hwaasTest {
  name = "Run fio disk benchmark";

  apiUrl = "http://api.hwaas.placeholder.com/v5";

  nodes.controlVM = { ... }:
    {
      imports = with hwaasTestModules; [
        user-tooling-hwaasTestVm
      ];

      environment.systemPackages = with pkgs; [ sshpass ];

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

  skipTypeCheck = true;

  testScript = _: ''
    import json
    import time
    from hwaas_driver import get_collector

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

    fio_call = """
      fio \
      --name=test \
      --rw=randread \
      --time_based \
      --runtime=10 \
      --output-format=json \
      --direct=1 \
      --size=1000M \
      --output-format=json \
      | tee result.json
    """

    start_all()
    controlVM.wait_for_unit("default.target")

    hwaas.machines["legacy-box"].power_on()

    wait_for_network()

    msg = machine.succeed(f"sshpass -p1234 ssh -oStrictHostKeyChecking=no nixos@192.168.44.2 '{fio_call}'");

    # The fio benchmark reports a timestamp that is not in the correct format for
    # ElasticSearch. The upload tooling adds a well formatted timestamp to each
    # document, if nothing is contained in the result.
    # We just remove the timestamp from the result and let the upload tooling add
    # it in the correct format. Using a different timestamp is OK, because the time
    # difference is very small between the actual test run and the upload.
    # Additionally, we can not guarantee that the system time of the systems under test
    # is configured correctly.
    data = json.loads(msg)
    del data["timestamp"]
    del data["timestamp_ms"]

    # FIO reports the benchmark result as a list of jobs.
    # Lists and arrays are not recommended to use with the analytics platform because
    # they are hard to index.
    # We have a single Fio job, thus we convert the list with one element to a single
    # job attribute.
    data["job"] = data["jobs"][0]
    del data["jobs"]

    # ! Warning: upload to ElasticSearch is currently omitted in this example !
    # benchmark_data_collector = get_collector()

    # The output format specification of the analytics platform requires a name
    # attribute in each document to create a detailed index.
    # metadata = { "name": "FIO" }

    # benchmark_data_collector.add_metadata(metadata)
    # benchmark_data_collector.add_result(data)
  '';
}
