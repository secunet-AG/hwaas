# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{
  runCommand,
  jd-diff-patch,
  remote-hands-oas,
  openapi-generator-cli,
}:
let
  expected = ../../../expected-oas;
in
runCommand "golden-test-openapi-spec" { inherit jd-diff-patch; } ''
  EXIT_CODE=0
  for FILE in ${expected}/remote-* ${remote-hands-oas}/*; do
    basename "$FILE"
  done | sort | uniq | while read FILE; do
    echo "Comparing OpenAPI spec $FILE"
    ${jd-diff-patch}/bin/jd -set ${expected}/$FILE ${remote-hands-oas}/$FILE
    CODE=$?
    if [ $CODE != 0 ]; then
      EXIT_CODE=$CODE
    fi
    echo "Validating OpenAPI spec $FILE"
    ${openapi-generator-cli}/bin/openapi-generator-cli validate -i ${remote-hands-oas}/$FILE
    CODE=$?
    if [ $CODE != 0 ]; then
      EXIT_CODE=$CODE
    fi
  done

  if [ $EXIT_CODE != 0 ]; then
    exit $EXIT_CODE
  else
    touch $out
  fi
''
