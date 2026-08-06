# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{
  stdenv,
  jq,
  pnpmConfigHook,
  vueClient,
  writeShellApplication,
  noticeFile,
}:

let
  checkLicenses = writeShellApplication {
    name = "check";
    runtimeInputs = [
      jq
      vueClient.pnpm
    ];
    # Leaving this separate for proper formatting and linting
    text = builtins.readFile ./vue-client-licenses-check.sh;
  };
in

stdenv.mkDerivation {
  pname = "vue-client-license-check";
  inherit (vueClient) version src pnpmDeps;

  nativeBuildInputs = [
    vueClient.node
    vueClient.pnpm
    pnpmConfigHook
    jq
  ];

  dontBuild = true;
  doCheck = true;

  checkPhase = ''
    runHook preCheck

    ${checkLicenses}/bin/check ${noticeFile} 2>&1 | tee license-report.txt

    runHook postCheck
  '';

  installPhase = ''
    runHook preInstall

    mkdir -p "$out"
    cp license-report.txt "$out/"

    runHook postInstall
  '';
}
