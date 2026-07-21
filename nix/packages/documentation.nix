# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ stdenv
, nodejs
, pnpm_10
, fetchPnpmDeps
, pnpmConfigHook
, documentationSrc
, hwaas-oas
,
}:

stdenv.mkDerivation (finalAttrs: {
  pname = "documentation";
  version = "0.0.1";

  src = documentationSrc;

  nativeBuildInputs = [
    nodejs
    pnpmConfigHook
    pnpm_10
  ];

  pnpmDeps = fetchPnpmDeps {
    inherit (finalAttrs) pname version src;
    pnpm = pnpm_10;
    fetcherVersion = 3;
    hash = "sha256-4YS5zZMFv98loIdBN007lqy4XoLvsQHjxPP5Ysp2C2U=";
  };

  preBuild = ''
    cp ${hwaas-oas} public/openapi.json
  '';

  installPhase = ''
    pnpm install
    pnpm build
    mkdir -p $out/dist
    cp -r ./dist $out
  '';
})
