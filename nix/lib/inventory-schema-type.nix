# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{
  check-jsonschema,
  writeTextFile,
  runCommand,
  configSchema,
  lib,
}:
appConfig:
let
  appConfigFile = writeTextFile {
    name = "configuration-json";
    text = builtins.toJSON appConfig;
  };
  validation = runCommand "check-config-schema" { } ''
    mkdir -p $out
    (${check-jsonschema}/bin/check-jsonschema --verbose --schemafile ${configSchema} ${appConfigFile} \
      > $out/log && echo $? > $out/code) || ( echo $? > $out/code && exit 0)
  '';
  log = lib.fileContents "${builtins.toString validation}/log";
  code = lib.fileContents "${builtins.toString validation}/code";
  success = code == "0";
in
lib.warnIf (!success) "check-config-schema failed: \n${log}" success
