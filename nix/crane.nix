# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ inputs, ... }: {
  perSystem =
    { pkgs, lib, ... }:
    let
      craneLib = (inputs.crane.mkLib pkgs).overrideToolchain (
        p: p.rust-bin.fromRustupToolchainFile ../rust-toolchain.toml
      );

      workspaceRoot = ../components;

      # The complete Rust workspace source.
      #
      # Deliberately use the same workspace source for every final package.
      # cargoExtraArgs selects which Cargo package gets built.
      #
      # This avoids having to construct partial Cargo workspaces and keeps the
      # Nix implementation considerably simpler.
      workspaceSrc = lib.fileset.toSource {
        root = workspaceRoot;

        fileset = lib.fileset.unions [
          # All normal Cargo/Rust sources, including Cargo.toml/Cargo.lock.
          (craneLib.fileset.commonCargoSources workspaceRoot)

          # Non-Rust inputs used by ContextAPI.
          ../components/contextapi/db_interaction/migrations
          ../components/contextapi/context_data_structures/src/network/patch/test_fixtures
        ];
      };

      # Inputs necessary for building *any* member of the workspace.
      #
      # Since buildDepsOnly builds dependencies for the entire workspace,
      # SQLite must be present here even though only ContextAPI needs it
      # directly.
      commonArgs = {
        src = workspaceSrc;

        cargoToml = ../components/Cargo.toml;
        cargoLock = ../components/Cargo.lock;

        strictDeps = true;

        # The root manifest is a virtual workspace, so provide explicit
        # derivation metadata instead of asking crane to infer a package.
        pname = "hwaas-workspace";
        version = "0.0.0";

        buildInputs = [
          pkgs.openssl
          pkgs.sqlite
        ]
        ++ lib.optionals pkgs.stdenv.isDarwin [
          # Additional darwin specific inputs can be set here
          pkgs.libiconv
          pkgs.darwin.apple_sdk.frameworks.Security
        ];

        nativeBuildInputs = [ pkgs.pkg-config ];
      };

      # Build the dependency graph exactly once and allow all later
      # derivations to reuse the resulting target directory.
      cargoArtifacts = craneLib.buildDepsOnly commonArgs;

      # Metadata for every package that is currently exported.
      #
      # manifestPath is used only for reading pname/version and generating
      # the per-package SBOM.
      packageDefinitions =
        let
          packagePaths = [
            "aruba-switch-mock"
            "hunt"
            "rpi-status-display"
            "net-ctrl"
            "ws-gateway"
            "ws-proxy-client"
            "remote-hands/remote-auxiliary"
            "remote-hands/remote-power"
            "remote-hands/remote-serial"
            "remote-hands/remote-usb"
            "contextapi/contextapi"
            "contextapi/machine_ops"
          ];
          # Format package paths into manifest attribute sets
          # Sets will be named like the package path.
          # For sub-paths, sets will be named like the last part of the path.
          mkSets =
            paths:
            builtins.listToAttrs (
              map (path: {
                name = lib.replaceStrings [ "_" ] [ "-" ] (lib.last (lib.splitString "/" path));
                value = rec {
                  manifest = ../components + "/${manifestPath}";
                  manifestPath = "${path}/Cargo.toml";
                };
              }) paths
            );
        in
        mkSets packagePaths;

      # Build one exported Cargo package.
      mkPackage =
        packageName:
        let
          definition = packageDefinitions.${packageName};

          crateInfo = craneLib.crateNameFromCargoToml { cargoToml = definition.manifest; };
        in
        craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;

            pname = packageName;
            inherit (crateInfo) version;

            # Select exactly one top-level Cargo package while retaining the
            # complete Cargo workspace as source.
            cargoExtraArgs = "-p ${packageName}";

            # Tests are done once for the entire workspace through nextest.
            doCheck = false;
          }
        );

      packages = lib.mapAttrs (packageName: _: mkPackage packageName) packageDefinitions;

      # One workspace-wide test/check set.
      workspaceChecks = {
        # Docs and doctests
        cargo-docs = craneLib.cargoDoc (
          commonArgs
          // {
            inherit cargoArtifacts;

            cargoExtraArgs = "--workspace";
            RUSTDOCFLAGS = "--deny warnings";
          }
        );

        # Clippy linting
        cargo-clippy = craneLib.cargoClippy (
          commonArgs
          // {
            inherit cargoArtifacts;
            # `--no-deps`: only lint workspace members and not their dependencies
            # `--exclude`: exclude `net_ctrl_client` since it is completely auto generated
            # `--deny`: fail on warnings
            cargoClippyExtraArgs = "--workspace --all-targets --exclude net_ctrl_client -- --no-deps --deny warnings";
          }
        );

        # NOTE: All formatting is taken care of by `nix fmt`

        # Clippy conformity
        cargo-nextest = craneLib.cargoNextest (
          commonArgs
          // {
            inherit cargoArtifacts;

            cargoNextestExtraArgs = "--workspace";

            partitions = 1;
            partitionType = "count";
            cargoNextestPartitionsExtraArgs = "--no-tests=pass";
          }
        );

        # Run cargo-deny
        cargo-deny =
          let
            denyConfig = ../components/deny.toml;
          in
          craneLib.cargoDeny (commonArgs // { cargoDenyExtraArgs = "--config ${denyConfig}"; });

        # Ensure the single workspace-hack remains synchronized with the
        # complete workspace dependency graph.
        cargo-hakari = craneLib.mkCargoDerivation (
          commonArgs
          // {
            pname = "hwaas-workspace-hakari";

            cargoArtifacts = null;
            doInstallCargoArtifacts = false;

            nativeBuildInputs = commonArgs.nativeBuildInputs ++ [ pkgs.cargo-hakari ];

            buildPhaseCargoCommand = ''
              cargo hakari generate --diff  # workspace-hack Cargo.toml is up-to-date
              cargo hakari manage-deps --dry-run  # all workspace crates depend on workspace-hack
              cargo hakari verify
            '';
          }
        );
      };

      # Preserve the existing sbom-<package> outputs.
      mkSbom =
        packageName:
        let
          definition = packageDefinitions.${packageName};
        in
        craneLib.mkCargoDerivation (
          commonArgs
          // {
            pname = "sbom-${packageName}";
            version = "3.1.0";

            # SBOM generation uses Cargo metadata rather than compiled
            # artifacts.
            cargoArtifacts = null;
            doInstallCargoArtifacts = false;
            doCheck = false;

            nativeBuildInputs = [
              pkgs.cargo-cyclonedx
              pkgs.cyclonedx-cli
            ];

            buildPhaseCargoCommand = ''
              cargo-cyclonedx cyclonedx --spec-version 1.5 -f json -v \
                --manifest-path ${definition.manifestPath}

              cyclonedx merge --output-file merged.cdx.json \
                --input-files $(find . -name "*.cdx.json") ||
                echo "WARNING: not merging - no files found"
            '';

            installPhaseCommand = ''
              mkdir "$out"
              find . -name "*.cdx.json" -exec cp -t $out {} +
            '';
          }
        );

      sbomPackages = lib.mapAttrs' (
        packageName: _: lib.nameValuePair "sbom-${packageName}" (mkSbom packageName)
      ) packageDefinitions;

      checks = workspaceChecks;

      # One development shell for the entire Rust workspace.
      devShells.rust = craneLib.devShell {
        # Inherit inputs from checks.
        inherit checks;

        # Extra inputs can be added here; cargo and rustc are provided by default.
        packages = with pkgs; [
          rust-analyzer
          cargo-watch
          cargo-audit
          cargo-cyclonedx
          cyclonedx-cli
          cargo-hakari

          # ContextAPI development.
          diesel-cli
          sqlite
        ];
      };
    in
    {
      config = {
        packages = packages // sbomPackages;

        inherit checks devShells;
      };
    };
}
