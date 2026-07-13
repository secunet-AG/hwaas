# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{
  switch1 = {
    ip = "127.0.0.1";
    port = null;
    model = "aruba";
    credentials = {
      username = "hwaas";
      password = "hwaas";
    };
    critical_ports = {
      mgmt_ports = [ "0" ];
      trunk_ports = [ "42" ];
    };
    default_vlan = {
      vlan_id = 1;
    };
    mgmt_vlan = {
      vlan_id = 1;
    };
  };
}
