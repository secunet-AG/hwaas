# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{
  runCommand,
  net-ctrl-openapi,
  openapi-generator-cli,
}:
runCommand "generate-net-ctrl-rust-client" { } ''
  mkdir -p $out
  ${openapi-generator-cli}/bin/openapi-generator-cli generate \
    -g rust \
    --additional-properties=packageName=net_ctrl_client,supportMiddleware=true \
    -i ${net-ctrl-openapi} \
    --skip-validate-spec \
    -o $out \
    --openapi-generator-ignore-list git_push.sh
''
