# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ runCommand
, makeWrapper
, lib
, jq
, httpie
, gnugrep
, writeShellScript
, image
, utils
,
}:
let
  wrapper = runCommand "wrap usb test" { nativeBuildInputs = [ makeWrapper ]; } ''
    makeWrapper ${./usb_test.sh} $out\
      --prefix PATH : ${
        lib.makeBinPath [
          jq
          httpie
          gnugrep
          utils
        ]
      }
  '';
in
writeShellScript "usb test" ''
  ${wrapper} "${image}" cidoka-terminal-server
''
