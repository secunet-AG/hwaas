# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

# A mock used to simulate a real ws-gateway.
{
  config,
  lib,
  pkgs,
  ...
}:
with lib;
let
  cfg = config.services.ws-gateway-mock;
  portString = builtins.toString cfg.port;
in
{
  imports = [ ./test-config.nix ];

  options.services.ws-gateway-mock = {
    enable = mkEnableOption "mocking ws-gateway";
    port = mkOption {
      type = types.port;
      default = 8234;
    };
  };

  config = mkIf cfg.enable {
    context-api-test-config = {
      ws_gateway_url = "ws://127.0.0.1:${portString}";
    };

    systemd.services.websocat-server = {
      description = "websocat server";
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];
      serviceConfig = {
        ExecStart = "${pkgs.websocat}/bin/websocat -s ${portString} -v";
      };
    };

    environment.systemPackages = [ pkgs.websocat ];
  };
}
