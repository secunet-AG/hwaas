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
      packageDefinitions = {
        aruba-switch-mock = {
          manifest = ../components/aruba-switch-mock/Cargo.toml;
          manifestPath = "aruba-switch-mock/Cargo.toml";
        };

        hunt = {
          manifest = ../components/hunt/Cargo.toml;
          manifestPath = "hunt/Cargo.toml";
        };

        rpi-status-display = {
          manifest = ../components/rpi-status-display/Cargo.toml;
          manifestPath = "rpi-status-display/Cargo.toml";
        };

        net-ctrl = {
          manifest = ../components/net-ctrl/Cargo.toml;
          manifestPath = "net-ctrl/Cargo.toml";
        };

        ws-gateway = {
          manifest = ../components/ws-gateway/Cargo.toml;
          manifestPath = "ws-gateway/Cargo.toml";
        };

        ws-proxy-client = {
          manifest = ../components/ws-proxy-client/Cargo.toml;
          manifestPath = "ws-proxy-client/Cargo.toml";
        };

        remote-auxiliary = {
          manifest = ../components/remote-hands/remote-auxiliary/Cargo.toml;
          manifestPath = "remote-hands/remote-auxiliary/Cargo.toml";
        };

        remote-power = {
          manifest = ../components/remote-hands/remote-power/Cargo.toml;
          manifestPath = "remote-hands/remote-power/Cargo.toml";
        };

        remote-serial = {
          manifest = ../components/remote-hands/remote-serial/Cargo.toml;
          manifestPath = "remote-hands/remote-serial/Cargo.toml";
        };

        remote-usb = {
          manifest = ../components/remote-hands/remote-usb/Cargo.toml;
          manifestPath = "remote-hands/remote-usb/Cargo.toml";
        };

        contextapi = {
          manifest = ../components/contextapi/contextapi/Cargo.toml;
          manifestPath = "contextapi/contextapi/Cargo.toml";
        };

        machine-ops = {
          manifest = ../components/contextapi/machine_ops/Cargo.toml;
          manifestPath = "contextapi/machine_ops/Cargo.toml";
        };
      };

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
        docs = craneLib.cargoDoc (
          commonArgs
          // {
            inherit cargoArtifacts;

            cargoExtraArgs = "--workspace";
          }
        );

        # Clippy conformity
        clippy = craneLib.cargoClippy (
          commonArgs
          // {
            inherit cargoArtifacts;
            # Only lint workspace members and not their dependencies
            # Additionally exclude `net_ctrl_client` since it is completely auto generated
            cargoClippyExtraArgs = "--workspace --all-targets --exclude net_ctrl_client -- --no-deps";
          }
        );

        # NOTE: All formatting is taken care of by `nix fmt`

        # Clippy conformity
        nextest = craneLib.cargoNextest (
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
        deny =
          let
            denyConfig = ../components/deny.toml;
          in
          craneLib.cargoDeny (commonArgs // { cargoDenyExtraArgs = "--config ${denyConfig}"; });

        # Ensure the single workspace-hack remains synchronized with the
        # complete workspace dependency graph.
        hakari = craneLib.mkCargoDerivation (
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
