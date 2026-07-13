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
    port = mkOption {
      type = types.port;
      default = 8765;
    };
  };

  config = mkIf cfg.enable {
    context-api-test-config = {
      net_ctrl_port = toString cfg.port;
      remote_power = "http://localhost:${toString cfg.port}/power";
    };

    services.http-echo-server = {
      enable = true;
      inherit (cfg) port;
    };
  };
}
