# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{
  stdenv,
  nodejs,
  pnpm_10,
  fetchPnpmDeps,
  pnpmConfigHook,
  vueSrc,
}:

stdenv.mkDerivation (finalAttrs: {
  pname = "vue-client";
  version = "0.0.0";

  src = vueSrc;

  nativeBuildInputs = [
    nodejs
    pnpmConfigHook
    pnpm_10
  ];

  pnpmDeps = fetchPnpmDeps {
    inherit (finalAttrs) pname version src;
    pnpm = pnpm_10;
    fetcherVersion = 4;
    hash = "sha256-2FI9/Sd4fanUAncW+nH7hoOIRK4xudZzzNBPqwZr6wQ=";
  };

  buildPhase = ''
    runHook preBuild
    pnpm build
    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall
    mkdir -p "$out"
    cp -r dist "$out/dist"
    runHook postInstall
  '';

  passthru = {
    inherit (finalAttrs) pnpmDeps;
    pnpm = pnpm_10;
    node = nodejs;
  };
})
