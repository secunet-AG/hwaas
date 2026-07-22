# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

_: {
  perSystem =
    { config, pkgs, ... }:
    let
      # Generate the job list for all stages
      jobs = import ./jobs.nix { inherit (pkgs) lib; inherit config; };
      # Generate CI yaml file from jobs list
      generator = import ./generate-ci.nix { inherit pkgs jobs; inherit (pkgs) lib; };
      # Verify staged/committed ci.yml is equal to a freshly build one with the generator
      verificator = import ./verify-ci.nix { inherit pkgs generatedWorkflow; };
      inherit (generator) generatedWorkflow;
    in
    {
      apps = {
        generate-ci = {
          type = "app";
          program = "${generator.generateCi}/bin/generate-ci";
        };
      };
      checks = {
        verify-ci = verificator;
      };
    };
}
