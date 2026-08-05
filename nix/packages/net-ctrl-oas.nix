# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ runCommand, net-ctrl }:
runCommand "generate-netctrl-openapi-json" { } ''
  ${net-ctrl}/bin/net-ctrl-openapi-generator --out-file $out
''
