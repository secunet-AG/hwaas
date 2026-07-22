# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ pkgs
, lib
, jobs
, system ? "x86_64-linux"
, workflowPath ? ".github/workflows/ci.yml"
, workflowName ? "CI"
, mainBranch ? "main"
, runner ? "ubuntu-latest"
, checkoutAction ? "actions/checkout@v7"
, installNixAction ? "cachix/install-nix-action@v31"
, cacheAction ? "cachix/cachix-action@v17"
,
}:

let
  # Create one step for a single target
  mkTargetStep =
    phase: target:
    let
      # Simplify the name of the step, since the job itself already specifies more
      step = lib.last (lib.splitString "." target);
    in
    {
      name = "${phase} ${step}";
      "if" = "always()";
      run = "nix build -L --show-trace ${lib.escapeShellArg target}";
    };

  # Steps that every job need to run once at the start
  commonSteps =
    [
      {
        name = "Check out repository";
        uses = checkoutAction;
      }
      {
        name = "Install Nix";
        uses = installNixAction;
        "with" = {
          nix_path = "nixpkgs=channel:nixos-unstable";
        };
      }
      {
        name = "Use cache";
        uses = cacheAction;
        "with" = {
          name = "secunet-ag-hwaas";
          authToken = "\${{ secrets.CACHIX_AUTH_TOKEN }}";
        };
      }
    ];

  # Create one job with multiple steps
  mkJob =
    job:
    {
      name = job.displayName;
      "runs-on" = runner;
      inherit (job) needs;
      steps =
        commonSteps
        ++ map
          (target: mkTargetStep job.phase target)
          job.targets;
    };

  # Map over all known jobs and create job structure
  generateJobs =
    builtins.listToAttrs (
      map
        (job: {
          name = job.id;
          value = mkJob job;
        })
        jobs
    );

  # Create workflow structure of yml file
  workflow = {
    name = workflowName;
    on = {
      push = {
        branches = [ mainBranch ];
      };
      pull_request = { };
      workflow_dispatch = { };
    };
    permissions = {
      contents = "read";
    };
    concurrency = {
      group = "ci-\${{ github.workflow }}-\${{ github.ref }}";
      cancel-in-progress = true;
    };
    jobs = generateJobs;
  };

  # Raw yml file
  rawWorkflow = (pkgs.formats.yaml { }).generate "ci-generated.yml" workflow;

  # Formatted yml file with "do not edit" header.
  # Used inside the `verify-ci` check to compare to staged/committed `ci.yml`.
  generatedWorkflow =
    pkgs.runCommand "generated-github-actions-workflow.yml"
      {
        nativeBuildInputs = [
          pkgs.coreutils
          pkgs.yamlfmt
        ];
      }
      ''
        {
          echo '# This file is generated. Do not edit it manually.'
          echo '#'
          echo '# Regenerate it with:'
          echo '#'
          echo '#   nix run .#generate-ci'
          echo
          cat ${rawWorkflow}
        } > "$out"

        yamlfmt "$out"
      '';

  # Actually update ci.yml in the repository.
  generateCi =
    pkgs.writeShellApplication {
      name = "generate-ci";

      runtimeInputs = [
        pkgs.coreutils
        pkgs.diffutils
        pkgs.git
      ];

      text = ''
        set -euo pipefail

        destination="${workflowPath}"
        destination_directory="$(dirname "$destination")"

        mkdir -p "$destination_directory"

        temporary_file="$(mktemp "$destination_directory/.ci.yml.XXXXXX")"

        cleanup() {
          rm -f "$temporary_file"
        }

        trap cleanup EXIT

        cp ${generatedWorkflow} "$temporary_file"
        chmod u+w "$temporary_file"

        # Verify if changes are even needed before actually updating the file.
        if [[ -f "$destination" ]] && cmp --silent "$temporary_file" "$destination"; then
          echo "$destination is already up to date."
          exit 0
        fi

        if [[ -f "$destination" ]]; then
          echo "Updating $destination"
          echo
          diff --unified "$destination" "$temporary_file" || true
          echo
        else
          echo "Creating $destination"
        fi

        mv "$temporary_file" "$destination"
        trap - EXIT

        echo
        echo "Done"
      '';
    };
in
{
  inherit
    generateCi
    generatedWorkflow
    ;
}
