# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ lib
, pkgs
, ...
}:
with lib;
{
  console.keyMap = "de";

  environment.systemPackages = with pkgs; [
    jq
    tmux
    tcpdump
    iputils
    pciutils
    websocat
    tokio-console
  ];

  # needed for tokio console
  systemd.services.context-api.environment = {
    RUST_LOG = "tokio=trace,runtime=trace";
  };

  networking.enableIPv6 = false;
  services.contextApi.consoleAddress = "127.0.0.1:6669";
}
