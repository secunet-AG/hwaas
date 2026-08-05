# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{
  runCommand,
  jd-diff-patch,
  contextapi-oas,
  openapi-generator-cli,
}:
let
  expected = ../../../expected-oas/contextapi.openapi.json;
in
runCommand "golden-test-openapi-spec" { inherit jd-diff-patch; } ''
  mkdir -p $out

  ${jd-diff-patch}/bin/jd -set ${expected} ${contextapi-oas}

  ${openapi-generator-cli}/bin/openapi-generator-cli validate -i ${contextapi-oas}
''
