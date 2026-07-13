# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ runCommand
, contextapi
,
}:
runCommand "generate-context-api-openapi-json" { } ''
  ${contextapi}/bin/openapi-generator --out-file $out
''
