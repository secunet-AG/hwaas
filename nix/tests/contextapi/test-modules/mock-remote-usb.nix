# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{
  config,
  lib,
  pkgs,
  ...
}:
with lib;
let
  cfg = config.services.mock-remote-usb;

  port = toString cfg.port;

  url = "http://127.0.0.1:${port}/usb";

  package = pkgs.runCommand "mock-remote-usb" { } ''
    mkdir $out
    cp ${./mock-remote-usb.py} $out/mock_remote_usb.py
  '';

in
{
  options.services.mock-remote-usb = {
    enable = mkEnableOption "Mocked remote-usb server";
    port = mkOption {
      type = types.int;
      default = 8081;
      # Not actually passed to the script
      readOnly = true;
    };
  };

  config = lib.mkIf cfg.enable {
    context-api-test-config = with lib; {
      remote_usb = mkDefault url;
    };

    systemd.services.mock-remote-usb = {
      description = "REST echo server";
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];
      before = [ "context-api.service" ];
      serviceConfig = {
        WorkingDirectory = toString package;
        ExecStart = "${pkgs.python3}/bin/python ${./mock-remote-usb.py}";
        # block until port is actually open
        ExecStartPost = ''
          ${pkgs.coreutils}/bin/timeout 5 ${pkgs.bash}/bin/bash -c '\
            while ! ${pkgs.iproute2}/bin/ss -H -t -l -n sport = :${port} \
              | ${pkgs.gnugrep}/bin/grep -q "^LISTEN.*:${port}"; do\
              ${pkgs.coreutils}/bin/sleep 1;\
            done'
        '';
      };
    };
  };
}
