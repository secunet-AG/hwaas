# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ runCommand, zizmor }:

runCommand "zizmor-check"
  {
    nativeBuildInputs = [ zizmor ];
    src = ../..;
  }
  ''
    cp -r "$src" repo
    chmod -R +w repo
    cd repo

    zizmor .

    touch "$out"
  ''
