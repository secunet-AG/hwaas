# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

# deadnix: skip
perSystem@{ config, ... }:
_: {
  systemd.services.http-sim = {
    enable = true;
    description = "HTTP Simulation server";
    wantedBy = [ "multi-user.target" ];
    after = [ "network.target" ];
    serviceConfig = {
      ExecStart = "${perSystem.config.packages.http-sim}/bin/server.py";
      Restart = "always";
      Type = "simple";
    };
  };
}
