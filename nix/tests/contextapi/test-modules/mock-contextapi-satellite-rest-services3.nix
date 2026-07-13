# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

# A mock used to simulate a real HWaaS services: remote-serial with websocket support + other external services.
# The remote-serial is realized by means of a reverse proxy that redirects websocket traffic
# to a dedicated websocket echo server. All other HTTP requests to the `port` setup here get redirected
# to an HTTP echo server.
{ config
, lib
, pkgs
, modules
, ...
}:
with lib;
let
  cfg = config.services.reverse-proxy;
  httpEchoPortString = builtins.toString cfg.port-http-echo;
  wsEchoPortString = builtins.toString cfg.port-ws;
in
{
  imports = [
    modules.test-restapi-echo-server
    ./test-config.nix
    ./websocket-echo-server.nix
  ];

  options.services.reverse-proxy = {
    enable = mkEnableOption "setup a reverse proxy over echo ws server and ts http api";
    port = mkOption {
      type = types.port;
      default = 8214;
    };

    port-http-echo = mkOption {
      type = types.port;
      default = 8765;
    };

    port-ws = mkOption {
      type = types.port;
      default = 8234;
    };
  };

  config = mkIf cfg.enable {

    context-api-test-config = {
      enable = true;
      net_ctrl_port = httpEchoPortString;
      remote_power = "http://localhost:${builtins.toString cfg.port}/power";
    };

    services = {
      ws-echo-server = {
        enable = true;
        port = cfg.port-ws;
      };

      http-echo-server = {
        enable = true;
        port = cfg.port-http-echo;
      };

      nginx = {
        enable = true;
        virtualHosts."http://localhost" = {
          listen = [
            {
              inherit (cfg) port;
              addr = "127.0.0.1";
              ssl = false;
            }
          ];
          extraConfig = ''
            location /serial {
              # Proxy to the websocket echo server when the url ends with /websocket
              location ~ \/websocket$ {
                proxy_pass http://127.0.0.1:${wsEchoPortString};
                proxy_http_version 1.1;
                proxy_set_header Upgrade $http_upgrade;
                proxy_set_header Connection $connection_upgrade;
              }
              # /serial* goes to the http echo server, unless it ends with /websocket
              # as that more specific case is prioritized and handled above
              proxy_pass http://127.0.0.1:${httpEchoPortString};
            }

            # When the path is exactly /serial then we return a hard coded json array
            # unless the method is different from GET, in which case we proxy to the
            # http echo server
            location = /serial {
              default_type application/json;
              return 200 '["0"]';
              # If the method is not GET we proxy to the http echo server
              limit_except GET {
                proxy_pass http://127.0.0.1:${httpEchoPortString};
              }
            }

            location / {
              # Anything that does not match something more specific is forwarded
              # to the http echo server
              proxy_pass http://127.0.0.1:${httpEchoPortString};
            }
          '';
        };
      };
    };

    systemd.services.reverse-proxy = {
      requires = [
        "echo-server.service"
        "ws-echo-server.service"
      ];
      after = [
        "echo-server.service"
        "ws-echo-server.service"
      ];

      serviceConfig = {
        ExecStart = "${pkgs.nginx}/bin/nginx";
      };
    };

    environment.systemPackages = [ pkgs.nginx ];
  };
}
