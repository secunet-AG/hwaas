# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

# This is the default ContextAPI config for all tests
# If a test needs a specific attr, make it an option of this module.

{ config
, lib
, modules
, ...
}:
with lib;
let
  cfg = config.context-api-test-config;
in
{
  imports = [
    modules.maintainer-cli
  ];

  options.context-api-test-config = {
    enable = lib.mkEnableOption "test ContextAPI config";

    store = mkOption {
      type = types.str;
      default = "/run/context-api/images";
    };

    db_file_path = mkOption {
      type = types.str;
      default = "/run/context-api/db.sqlite";
    };

    remote_auxiliary = mkOption {
      type = with types; nullOr str;
      default = null;
    };

    remote_power = mkOption {
      type = types.str;
      default = "http://192.168.100.1/power";
    };

    remote_serial = mkOption {
      type = with types; nullOr str;
      default = null;
    };

    remote_usb = mkOption {
      type = types.str;
      default = "http://192.168.100.1/usb";
    };

    net_ctrl_port = mkOption {
      type = types.str;
      default = "8765";
    };

    ws_gateway_url = mkOption {
      type = types.str;
      default = "ws://127.0.0.1:8234";
    };

    switch = mkOption {
      type = types.str;
      default = "switch1";
    };

    networkIdsEnd = mkOption {
      type = types.int;
      default = 4;
    };

    switch_connection_1 = mkOption {
      type = types.str;
      default = "lan1";
    };

    switch_connection_2 = mkOption {
      type = types.str;
      default = "lan2";
    };
  };

  config = mkIf cfg.enable {
    systemd.services.maintainer-cli-init-service = {
      before = [ "context-api.service" ];
      after = [ "echo-server.service" ];
    };
    services = {
      maintainerCli = {
        enable = true;
        configMachines = [
          {
            id = 1;
            switch_connections = {
              "${cfg.switch_connection_1}" = {
                inherit (cfg) switch;
                port = "2";
              };
              "${cfg.switch_connection_2}" = {
                inherit (cfg) switch;
                port = "3";
              };
            };
            inherit (cfg)
              remote_auxiliary
              remote_power
              remote_serial
              remote_usb
              ;
            platform = "";
          }
        ];
        configNetworks = lib.lists.range 3 cfg.networkIdsEnd;
      };

      contextApi.config = {
        inherit (cfg) db_file_path;
        net_ctrl_base_path = "http://localhost:${cfg.net_ctrl_port}";
        image_api_settings = {
          # The permissions for the store path must suit the contextapi user
          inherit (cfg) store;
          max_file_size = "128Mib";
        };
        network_gateway = {
          inherit (cfg) ws_gateway_url;
        };
      };
    };
  };
}
