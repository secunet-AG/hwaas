# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

# A mock used to simulate real HWaaS services (like a Terminal Server or NetCtrl)
{ config
, lib
, modules
, ...
}:
with lib;
let
  cfg = config.services.mock-contextapi-satellite-rest-services;
in
{
  imports = [
    ./test-config.nix
    modules.test-restapi-echo-server
  ];

  options.services.mock-contextapi-satellite-rest-services = {
    enable = mkEnableOption "Mock services by echo server";
    port-ts = mkOption {
      type = types.port;
      default = 8765;
    };
  };

  config = mkIf cfg.enable {

    context-api-test-config = {
      enable = true;
      remote_power = "http://127.0.0.1:${builtins.toString cfg.port-ts}/power";
    };

    services.http-echo-server = {
      enable = true;
      port = cfg.port-ts;
    };

    services.netCtrl = {
      enable = true;
      port = lib.toInt config.context-api-test-config.net_ctrl_port;
      inventory = {
        "${config.context-api-test-config.switch}" = {
          ip = "127.0.0.1";
          model = "dummy";
          credentials = {
            username = "foo";
            password = "bar";
          };
          critical_ports = {
            mgmt_ports = [ "1" ];
            trunk_ports = [
              "49"
              "50"
              "51"
              "52"
            ];
          };
          default_vlan = {
            vlan_id = 1;
          };
          mgmt_vlan = {
            vlan_id = 2;
          };
        };
      };
    };

    # overwriting context-api to wait for the echo server
    systemd.services.context-api = {
      requires = [
        "echo-server.service"
        "net-ctrl.service"
      ];
      after = [
        "echo-server.service"
        "net-ctrl.service"
      ];
    };
  };
}
