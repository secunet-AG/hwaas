# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ runCommand
, remote-serial
, remote-power
, remote-usb
, remote-auxiliary
,
}:
runCommand "generate-remote-hands-openapi-json"
{
  nativeBuildInputs = [
    remote-serial
    remote-power
    remote-usb
    remote-auxiliary
  ];
}
  ''
    mkdir -p $out
    remote-serial-openapi-generator > $out/remote-serial.openapi.json
    remote-power-openapi-generator > $out/remote-power.openapi.json
    remote-usb-openapi-generator > $out/remote-usb.openapi.json
    remote-auxiliary-openapi-generator > $out/remote-auxiliary.openapi.json
  ''
