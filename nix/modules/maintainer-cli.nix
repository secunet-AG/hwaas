# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

# deadnix: skip
perSystem@{ config }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  maintainerCliCfg = config.services.maintainerCli;
  contextApiConfig = config.services.contextApi.config;
  username = "contextApi";

  contextApiConfigFile = pkgs.writeText "contextAPI configuration.json" (
    builtins.toJSON contextApiConfig
  );
  configFileMachines = pkgs.writeText "maintainerCLI machines-configuration.json" (
    builtins.toJSON maintainerCliCfg.configMachines
  );
  configFileNetworks = pkgs.writeText "maintainerCLI networks-configuration.json" (
    builtins.toJSON maintainerCliCfg.configNetworks
  );

  initTaskScript = pkgs.writeShellScript "run-maintainer-tool" ''
    ${maintainerCliCfg.package}/bin/machine-ops initialize-machines run \
      -vv \
      --machines-file ${configFileMachines} \
      -c ${contextApiConfigFile} \
      --machine-reset-timeout "300" \
      --ignore-in-use && \
      ${maintainerCliCfg.package}/bin/machine-ops insert-network-ids run \
        --network-ids-file ${configFileNetworks} \
        --database ${maintainerCliCfg.db_file_path}
  '';

  maintainer-cli-config-schema-init-machines = pkgs.runCommand "generate-config-schema" { } ''
    ${maintainerCliCfg.package}/bin/machine-ops initialize-machines print-schema > $out
  '';
  maintainer-cli-config-schema-init-net = pkgs.runCommand "generate-config-schema" { } ''
    ${maintainerCliCfg.package}/bin/machine-ops insert-network-ids print-schema > $out
  '';
in
{
  options.services.maintainerCli = {
    enable = lib.mkEnableOption "the maintainer CLI startup task";

    package = lib.mkOption {
      default = perSystem.config.packages.machine-ops;
      type = lib.types.package;
      description = ''
        The contextapi package to use (contains the Maintainer CLI). This defaults to the release version.
      '';
    };

    configMachines = lib.mkOption {
      type = lib.types.addCheck lib.types.anything (
        pkgs.callPackage ../lib/inventory-schema-type.nix {
          configSchema = maintainer-cli-config-schema-init-machines;
        }
      );
      example = "/user/config-machines.json";
      # TODO: FIXME:
      description = "The path to the file containing the configuration in JSON.";
    };
    configNetworks = lib.mkOption {
      type = lib.types.addCheck lib.types.anything (
        pkgs.callPackage ../lib/inventory-schema-type.nix {
          configSchema = maintainer-cli-config-schema-init-net;
        }
      );
      example = "/user/config-net.json";
      # TODO: FIXME:
      description = "The path to the file containing the configuration in JSON.";
    };
    db_file_path = lib.mkOption {
      type = lib.types.str;
      default = "/run/context-api/db.sqlite";
    };
  };

  config = lib.mkIf maintainerCliCfg.enable {

    environment.systemPackages = [ maintainerCliCfg.package ];

    systemd.services.maintainer-cli-init-service = {
      description = "HWaaS Maintainer CLI init task";
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        Type = "oneshot";
        User = username;
        ExecStart = "${initTaskScript}";
        TimeoutStartSec = "30min";
      };
    };
  };
}
