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
    fetcherVersion = 3;
    hash = "sha256-qdJHn4HVRKezgvlVzMw2HCF9jRgtjo7oBfSR7qJa1xw=";
  };

  installPhase = ''
    pnpm install
    pnpm build
    mkdir -p $out/dist
    cp -r ./dist $out
  '';
})
