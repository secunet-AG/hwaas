# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ config
, lib
, pkgs
, ...
}:
with lib;
let
  cfg = config.services.http-echo-server;

  port = toString cfg.port;

  restapi-echo-server-src = pkgs.fetchFromGitHub {
    owner = "kthrdei";
    repo = "restapi-echo-server";
    rev = "ddbe193ccfebe2a2d37731b861ed1f1121313038";
    sha256 = "sha256-1FxEsEyilAUh3/P7pX1X82k4nSh7xNb0DAlQa5N/jj0=";
  };

in
{
  options.services.http-echo-server = {
    enable = mkEnableOption "HTTP echo server";
    port = mkOption {
      type = types.int;
      default = 8080;
    };
    bodyOnly = mkOption {
      type = types.bool;
      default = false;
    };
    url = mkOption {
      type = types.str;
      default = "http://localhost:${port}";
      readOnly = true;
    };
  };

  config = lib.mkIf cfg.enable {
    systemd.services.echo-server = {
      description = "REST echo server";
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];
      before = [ "context-api.service" ];
      serviceConfig = {
        WorkingDirectory = "${restapi-echo-server-src}/src";
        ExecStart =
          "${pkgs.python311}/bin/python -m restapi_echo_server --port ${port}"
          + lib.optionalString cfg.bodyOnly " -b";
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
