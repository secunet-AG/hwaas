# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{
  runCommand,
  contextapi,
  remote-hands-oas,
}:
runCommand "generate-hwaas-openapi-json" { } ''
  ARGS=""
  for F in ${remote-hands-oas}/*.json; do
    ARGS="$ARGS --remote-oas-paths $F"
  done

  ${contextapi}/bin/openapi-generator \
    -vv $ARGS \
    --out-file $out

  # Give the merged OAS spec a new title, since it is not the Context API alone
  sed -i -e 's/HWaaS Context API/HWaaS API/g' $out
''
