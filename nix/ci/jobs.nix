# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{
  lib,
  config,
  system ? "x86_64-linux",
  ...
}:

let
  # Jobs to exclude, need to match the full target string
  excludeJobs = [
    # Is already part of pre-commit check, does not need to be executed twice.
    ".#checks.${system}.treefmt"
    # Do not add the `-debug` packages that are only for local debugging of the same tests.
    ".#packages.${system}.contextapi-cors-test-debug"
    ".#packages.${system}.contextapi-images-drives-and-middleware-debug"
    ".#packages.${system}.contextapi-net-api-test-debug"
    ".#packages.${system}.contextapi-remote-hands-aux-device-test-debug"
    ".#packages.${system}.contextapi-remote-hands-routing-test-debug"
    ".#packages.${system}.contextapi-remote-serial-test-debug"
    ".#packages.${system}.contextapi-startup-test-debug"
    ".#packages.${system}.contextapi-test-open-telemetry-debug"
    ".#packages.${system}.contextapi-ws-network-routing-test-debug"
    ".#packages.${system}.ws-gateway-test-clients-and-suts-scenario-debug"
    ".#packages.${system}.ws-gateway-test-connect-debug"
    ".#packages.${system}.ws-gateway-test-many-clients-scenario-debug"
    ".#packages.${system}.ws-gateway-test-many-vlans-ping-debug"
  ];

  # Jobs will be grouped with these keywords
  groups = [
    "aruba-switch-mock"
    "contextapi"
    "hunt"
    "net-ctrl"
    "pre-commit"
    # Can't use remote-hands here, since jobs are called remote-power for example as well
    "remote"
    "rpi-status-display"
    "sbom"
    "user-tooling"
    "verify-ci"
    "ws-gateway"
  ];

  # Jobs ending with these keywords are regular checks and should run before the build.
  # Everything else is a test and runs after the build.
  checkTypes = [
    "checkTestconfig"
    "clippy"
    "docs"
    "fmt"
    "golden-test-openapi-spec"
    "hakari"
    "nextest"
    "pre-commit"
    "verify-ci"
  ];

  # A target belongs to a group when it's either exactly the group name or begins with it
  belongsToGroup = group: target: target == group || lib.hasPrefix "${group}-" target;

  findGroup = target: lib.findFirst (group: belongsToGroup group target) null groups;

  # Check whether the target ends in one of the explicitly configured check types.
  isRegularCheck =
    target: lib.any (checkType: target == checkType || lib.hasSuffix "-${checkType}" target) checkTypes;

  # Check whether a target is meant to be excluded.
  isExcluded = item: builtins.elem item.target excludeJobs;

  # We generally want to keep the order of "minimal checks" -> "checks" -> "builds" -> "tests".
  # Minimal checks should be as fast as possible and guarantee the sanity of the CI.
  # This is the main point for defining which jobs belong to the "minimal checks".
  #
  # Since we separate each stage into multiple groups, we could generate "empty" jobs.
  # For example: there is no "check" that the SBOM "build" job directly depends on.
  # This "build" job should still depend on the "minimal checks" though.
  #
  # This function simply returns the minimal checks list if the given needs list is empty.
  # Other options (making everything depend on them) would make the CI overview cumbersome.
  dependOnMinimalChecksIfEmpty =
    needs:
    if needs == [ ] then
      [
        "check-pre-commit"
        "check-verify-ci"
      ]
    else
      needs;

  # Normalize checks and packages into the same intermediate representation.
  checks = lib.filter (item: !isExcluded item) (
    lib.mapAttrsToList (name: _: {
      inherit name;

      group = findGroup name;
      target = ".#checks.${system}.${name}";
      kind = if isRegularCheck name then "check" else "test";
    }) config.checks
  );
  packages = lib.filter (item: !isExcluded item) (
    lib.mapAttrsToList (name: _: {
      inherit name;
      group = findGroup name;
      target = ".#packages.${system}.${name}";
      kind = "build";
    }) config.packages
  );

  # Produce one job per group and phase only when the target list is non-empty.
  # Returning null for an empty phase makes it easy to filter missing jobs
  # afterward, see `jobsForGroup`.
  mkJob =
    {
      id,
      displayName,
      phase,
      items,
      needs ? [ ],
      group ? null,
    }:
    if items == [ ] then
      null
    else
      {
        inherit
          displayName
          group
          id
          needs
          phase
          ;
        targets = map (item: item.target) items;
      };

  # Anything that matches a known group is collected into one
  # additional job for each phase.
  checksForGroup = group: builtins.filter (item: item.group == group && item.kind == "check") checks;

  testsForGroup = group: builtins.filter (item: item.group == group && item.kind == "test") checks;

  packagesForGroup = group: builtins.filter (item: item.group == group) packages;

  # Generate: check-<group> --> build-<group> --> test-<group>
  # Missing phases are skipped (when a group has no packages,
  # its test job depends directly on its check job).
  jobsForGroup =
    group:
    let
      checkJob = mkJob {
        id = "check-${group}";
        displayName = "Check: ${group}";
        phase = "check";
        inherit group;
        items = checksForGroup group;
        needs = lib.optionals (group != "pre-commit" && group != "verify-ci") (
          dependOnMinimalChecksIfEmpty [ ]
        );
      };

      buildJob = mkJob {
        id = "build-${group}";
        displayName = "Build: ${group}";
        phase = "build";
        inherit group;
        items = packagesForGroup group;
        needs = dependOnMinimalChecksIfEmpty (lib.optional (checkJob != null) checkJob.id);
      };

      testJob = mkJob {
        id = "test-${group}";
        displayName = "Test: ${group}";
        phase = "test";
        inherit group;
        items = testsForGroup group;
        needs = dependOnMinimalChecksIfEmpty (
          if buildJob != null then [ buildJob.id ] else lib.optional (checkJob != null) checkJob.id
        );
      };
    in
    builtins.filter (job: job != null) [
      checkJob
      buildJob
      testJob
    ];

  groupedJobs = lib.concatMap jobsForGroup groups;

  # Anything that does not match a known group is collected into one
  # additional job for each phase.
  # Rest same as above.
  ungroupedChecks = builtins.filter (item: item.group == null && item.kind == "check") checks;

  ungroupedTests = builtins.filter (item: item.group == null && item.kind == "test") checks;

  ungroupedPackages = builtins.filter (item: item.group == null) packages;

  ungroupedJobs =
    let
      checkJob = mkJob {
        id = "check-ungrouped";
        displayName = "Check: ungrouped";
        phase = "check";
        group = null;
        items = ungroupedChecks;
        needs = dependOnMinimalChecksIfEmpty [ ];
      };

      buildJob = mkJob {
        id = "build-ungrouped";
        displayName = "Build: ungrouped";
        phase = "build";
        group = null;
        items = ungroupedPackages;
        needs = dependOnMinimalChecksIfEmpty (lib.optional (checkJob != null) checkJob.id);
      };

      testJob = mkJob {
        id = "test-ungrouped";
        displayName = "Test: ungrouped";
        phase = "test";
        group = null;
        items = ungroupedTests;
        needs = dependOnMinimalChecksIfEmpty (
          if buildJob != null then [ buildJob.id ] else lib.optional (checkJob != null) checkJob.id
        );
      };
    in
    builtins.filter (job: job != null) [
      checkJob
      buildJob
      testJob
    ];

in
groupedJobs ++ ungroupedJobs
