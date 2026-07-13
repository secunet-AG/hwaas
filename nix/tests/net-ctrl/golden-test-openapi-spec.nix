# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ runCommand
, jd-diff-patch
, oas
,
}:
let
  expected-oas = ../../../expected-oas/net-ctrl.openapi.json;
in
runCommand "golden-test-openapi-spec" { inherit jd-diff-patch; } ''
  mkdir -p $out

  ${jd-diff-patch}/bin/jd -set ${expected-oas} ${oas}
''
