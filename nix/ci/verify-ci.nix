# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{
  pkgs,
  generatedWorkflow,
  workflowPath ? ../../.github/workflows/ci.yml,
}:

pkgs.runCommand "verify-ci"
  {
    nativeBuildInputs = [
      pkgs.coreutils
      pkgs.diffutils
    ];
  }
  ''
    set -euox pipefail

    committed_workflow="${workflowPath}"
    generated_workflow="${generatedWorkflow}"

    if [[ ! -f "$committed_workflow" ]]; then
      cat >&2 <<'EOF'
    The generated GitHub Actions workflow is missing.

    Generate it with:

      nix run .#generate-ci
    EOF

      exit 1
    fi

    if ! diff --unified "$committed_workflow" "$generated_workflow"; then
      cat >&2 <<'EOF'

    The committed GitHub Actions workflow is out of date.

    Regenerate the workflow with:

      nix run .#generate-ci
    EOF

      exit 1
    fi

    mkdir -p "$out"
    touch "$out/passed"
  ''
