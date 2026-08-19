# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ inputs, ... }: {
  perSystem =
    {
      pkgs,
      config,
      lib,
      ...
    }:
    let
      cfg = config.hwaas-crates;
      craneLib = (inputs.crane.mkLib pkgs).overrideToolchain (
        p: p.rust-bin.fromRustupToolchainFile ../rust-toolchain.toml
      );

      # per crate variables and processed information
      perProject =
        project:
        let
          # filter the sources for building a single crate.
          src = lib.fileset.toSource {
            root = ../components;
            fileset = lib.fileset.unions (
              (map (x: craneLib.fileset.commonCargoSources x) project.sources) ++ project.unfilteredSources
            );
          };

          cargoToml = "${project.baseCratePath}/Cargo.toml";
          cargoLock = "${project.baseCratePath}/Cargo.lock";

          # Common crane arguments can be set here to avoid repeating them later
          commonArgs = {
            strictDeps = true;
            inherit src cargoToml cargoLock;
            buildInputs = [
              pkgs.openssl
            ]
            ++ lib.optionals pkgs.stdenv.isDarwin [
              # Additional darwin specific inputs can be set here
              pkgs.libiconv
              pkgs.darwin.apple_sdk.frameworks.Security
            ]
            ++ project.extraDeps;

            nativeBuildInputs = [ pkgs.pkg-config ] ++ project.extraNativeDeps;

            postUnpack = ''
              cd $sourceRoot/${builtins.baseNameOf project.baseCratePath}
              sourceRoot="."
            '';
            # Additional environment variables can be set directly
            # MY_CUSTOM_VAR = "some value";
          };

          # Build *just* the cargo dependencies (of the entire workspace),
          # so we can reuse all of that work (e.g. via cachix) when running in CI
          # It is *highly* recommended to use something like cargo-hakari to avoid
          # cache misses when building individual top-level-crates
          cargoArtifacts = craneLib.buildDepsOnly commonArgs;

          individualCrateArgs = commonArgs // {
            inherit cargoArtifacts;
            inherit (craneLib.crateNameFromCargoToml { inherit cargoToml; }) version;
            # NB: we disable tests since we'll run them all via cargo-nextest
            doCheck = false;
          };
        in
        {
          inherit
            commonArgs
            individualCrateArgs
            src
            cargoArtifacts
            ;
        };

      genPerProject =
        fn:
        lib.concatMapAttrs (
          projectName: projectValue:
          let
            perProjectCfg = perProject projectValue;
          in
          fn { inherit projectName projectValue perProjectCfg; }
        ) cfg.project;

      genPerTarget =
        fn:
        genPerProject (
          {
            projectName,
            projectValue,
            perProjectCfg,
          }:
          lib.genAttrs projectValue.packages (
            packageName:
            (fn {
              inherit
                projectName
                projectValue
                packageName
                perProjectCfg
                ;
            })
          )
        );

      prefixAttrNames =
        prefix: lib.attrsets.mapAttrs' (n: v: lib.attrsets.nameValuePair ("${prefix}-" + n) v);

      checks = genPerProject (
        {
          perProjectCfg,
          projectName,
          projectValue,
          ...
        }:
        (prefixAttrNames projectName (
          {
            # Docs and doctests
            docs = craneLib.cargoDoc (perProjectCfg.commonArgs // { inherit (perProjectCfg) cargoArtifacts; });

            # Clippy conformity
            clippy = craneLib.cargoClippy (
              perProjectCfg.commonArgs
              // {
                inherit (perProjectCfg) cargoArtifacts;
                cargoClippyExtraArgs = "--all-targets -- --deny warnings";
              }
            );

            # NOTE: All formatting is taken care of by `nix fmt`

            # Run tests with cargo-nextest
            nextest = craneLib.cargoNextest (
              perProjectCfg.commonArgs
              // {
                inherit (perProjectCfg) cargoArtifacts;
                partitions = 1;
                partitionType = "count";
                cargoNextestPartitionsExtraArgs = "--no-tests=pass";
                cargoNextestExtraArgs = lib.optionalString projectValue.hasWorkspaces "--workspace";
              }
            );

            # Run cargo-deny
            deny =
              let
                denyConfig = ../components/deny.toml;
              in
              craneLib.cargoDeny (
                perProjectCfg.commonArgs
                // {
                  inherit (perProjectCfg) src;
                  cargoDenyExtraArgs = "--config ${denyConfig}";
                }
              );
          }
          // lib.optionalAttrs projectValue.hasWorkspaces {

            # Ensure that cargo-hakari is up to date
            hakari = craneLib.mkCargoDerivation (
              perProjectCfg.commonArgs
              // {
                inherit (perProjectCfg) src;
                pname = "${projectName}-hakari";
                cargoArtifacts = null;
                doInstallCargoArtifacts = false;

                buildPhaseCargoCommand = ''
                  cargo hakari generate --diff  # workspace-hack Cargo.toml is up-to-date
                  cargo hakari manage-deps --dry-run  # all workspace crates depend on workspace-hack
                  cargo hakari verify
                '';

                nativeBuildInputs = [ pkgs.cargo-hakari ];
              }
            );

          }
        ))
      );

      devShells = genPerProject (
        { projectName, projectValue, ... }: {
          ${projectName} = craneLib.devShell {
            # Inherit inputs from checks.
            checks = lib.filterAttrs (n: _: lib.hasPrefix projectName n) checks;

            # Extra inputs can be added here; cargo and rustc are provided by default.
            packages =
              with pkgs;
              [
                rust-analyzer
                cargo-watch
                cargo-audit
                cargo-cyclonedx
                cyclonedx-cli
              ]
              ++ (lib.optional projectValue.hasWorkspaces [ pkgs.cargo-hakari ])
              ++ projectValue.extraDepsDevShell;
          };
        }
      );

      # packages for all needed binaries
      packages =
        (genPerTarget (
          { packageName, perProjectCfg, ... }:
          craneLib.buildPackage (
            perProjectCfg.individualCrateArgs
            // {
              pname = packageName;
              cargoExtraArgs = "-p ${packageName}";
              inherit (perProjectCfg) src;
            }
          )
        ))
        # Add sbom package for all binaries
        // (prefixAttrNames "sbom" (
          genPerTarget (
            { packageName, perProjectCfg, ... }:
            craneLib.mkCargoDerivation (
              perProjectCfg.individualCrateArgs
              // {
                pname = "sbom-${packageName}";
                version = "3.1.0";
                cargoArtifacts = null;
                doInstallCargoArtifacts = false;
                doCheck = false;
                nativeBuildInputs = [
                  pkgs.cargo-cyclonedx
                  pkgs.cyclonedx-cli
                ];
                buildPhaseCargoCommand = ''
                  cargo-cyclonedx cyclonedx --spec-version 1.5 -f json -v
                  cyclonedx merge --output-file merged.cdx.json \
                    --input-files $(find . -name "*.cdx.json") ||
                    echo "WARNING: not merging - no files found"
                '';
                installPhaseCommand = ''
                  mkdir $out
                  find . -name "*.cdx.json" -exec cp -t $out {} +
                '';
              }
            )
          )
        ));

    in
    {
      options.hwaas-crates = {
        project = lib.mkOption {
          type = lib.types.attrsOf (
            lib.types.submodule (
              { config, name, ... }: {
                options = {
                  sources = lib.mkOption {
                    type = lib.types.nonEmptyListOf lib.types.path;
                    description = "all sources are filtered; only the listed one are used to build the crate";
                  };
                  unfilteredSources = lib.mkOption {
                    type = lib.types.listOf lib.types.path;
                    default = [ ];
                    description = "additional unfiltered sources; e.g. for sql migrations";
                  };
                  packages = lib.mkOption {
                    type = lib.types.nonEmptyListOf lib.types.str;
                    description = "name of all packages in this project to build";
                    default = [ name ];
                  };
                  baseCratePath = lib.mkOption {
                    type = lib.types.path;
                    default = builtins.elemAt config.sources 0;
                    readOnly = true;
                  };
                  hasWorkspaces = lib.mkOption {
                    type = lib.types.bool;
                    default = true;
                    description = "enables the workspace build and some checks";
                  };
                  extraDeps = lib.mkOption {
                    default = [ ];
                    type = lib.types.listOf lib.types.package;
                    description = "additional runtime packages";
                  };
                  extraNativeDeps = lib.mkOption {
                    default = [ ];
                    type = lib.types.listOf lib.types.package;
                    description = "additional build time packages";
                  };
                  extraDepsDevShell = lib.mkOption {
                    default = [ ];
                    type = lib.types.listOf lib.types.package;
                    description = "additional devshell packages";
                  };
                };
              }
            )
          );
          description = "configuration to build the crate";
          example = { };
        };
      };

      config = {
        hwaas-crates.project = {
          aruba-switch-mock.sources = [ ../components/aruba-switch-mock ];

          hunt = {
            sources = [ ../components/hunt ];
            hasWorkspaces = false;
          };

          rpi-status-display = {
            sources = [ ../components/rpi-status-display ];
            hasWorkspaces = false;
          };

          net-ctrl.sources = [
            ../components/net-ctrl
            ../components/hunt
          ];

          ws-gateway = {
            sources = [
              ../components/ws-gateway
              ../components/hunt
            ];
            packages = [
              "ws-gateway"
              "ws-proxy-client"
            ];
          };

          remote-hands = {
            sources = [
              ../components/remote-hands
              ../components/hunt
            ];
            packages = [
              "remote-auxiliary"
              "remote-power"
              "remote-serial"
              "remote-usb"
            ];
          };

          contextapi = {
            sources = [
              ../components/contextapi
              ../components/hunt
              ../components/remote-hands
            ];
            unfilteredSources = [
              ../components/contextapi/db_interaction/migrations
              ../components/contextapi/context_data_structures/src/network/patch/test_fixtures
            ];
            packages = [
              "contextapi"
              "machine-ops"
            ];
            extraDeps = [ pkgs.sqlite ];
            extraDepsDevShell = [ pkgs.diesel-cli ];
          };
        };

        inherit checks devShells packages;
      };
    };
}
