# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ writeShellScript
, curl
, contextApi
, context-api-url-version-prefix
,
}:
let
  # This script relies on environment variables:
  # * DEPENDECY_TRACK_URL: upload url
  # * DEPENDECY_TRACK_KEY: upload key
  sbom = "${contextApi.cycloneDX}/contextapi.cdx.json";
  program = writeShellScript "upload-sbom" ''
    ${curl}/bin/curl -vvk --fail -X "POST" "$DEPENDECY_TRACK_URL/api/v1/bom" \
      -H 'Content-Type: multipart/form-data' \
      -H "X-Api-Key: $DEPENDECY_TRACK_KEY" \
      -F "autoCreate=true" \
      -F "projectName=contextAPI" \
      -F "projectVersion=${context-api-url-version-prefix}" \
      -F "parentName=contextAPI" \
      -F "bom=@${sbom}"
  '';

in
{
  type = "app";
  program = "${program}";
  meta.description = "Upload the SBOM to Dependency Track";
}
