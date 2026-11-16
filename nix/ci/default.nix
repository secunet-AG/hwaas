# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

_: {
  perSystem =
    { config, pkgs, ... }:
    let
      # Generate the job list for all stages
      jobs = import ./jobs.nix {
        inherit (pkgs) lib;
        inherit config;
      };
      # Generate CI yaml file from jobs list
      generator = import ./generate-ci.nix {
        inherit pkgs jobs;
        inherit (pkgs) lib;
      };
      inherit (generator) generatedWorkflow;
    in
    {
      apps = {
        generate-ci = {
          type = "app";
          program = "${generator.generateCi}/bin/generate-ci";
          meta.description = "Generate ci.yml file for GitHub";
        };
      };
      checks = {
        # Verify staged/committed ci.yml is equal to a freshly build one with the generator
        verify-ci-generation = pkgs.callPackage ./verify-ci.nix { inherit generatedWorkflow; };
        # Static analysis of all GitHub CI .yml files
        verify-ci-zizmor = pkgs.callPackage ./zizmor.nix { };
      };
    };
}
