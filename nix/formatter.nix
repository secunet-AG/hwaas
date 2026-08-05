# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ inputs, ... }: {
  imports = [ inputs.treefmt-nix.flakeModule ];
  perSystem = _: {
    treefmt = {
      projectRootFile = "flake.nix";
      # Full list of supported formatters:
      # <https://github.com/numtide/treefmt-nix#supported-programs>
      programs = {
        nixfmt = {
          enable = true;
          strict = true;
          width = 100;
          indent = 2;
        };
        prettier = {
          enable = true;
          settings = {
            editorconfig = true;
            singleQuote = true;
            semi = false;
          };
        };
        rustfmt = {
          enable = true;
          edition = "2024";
        };
        shfmt = {
          enable = true;
          useEditorConfig = true;
        };
        taplo.enable = true;
        yamlfmt.enable = true;
      };
      settings = {
        formatter.prettier = {
          excludes = [
            "vue-client/pnpm-lock.yaml"
            "**/*.yml"
            "**/*.yaml"
          ];
        };
        global.excludes = [
          "^.cargo/.+$"
          "components/aruba-switch-mock/reference_schemars/**"
          "components/contextapi/net_ctrl_client/**"
          "**/*workspace-hack/Cargo.toml"
          "expected-oas/**"
          "vue-client/pnpm-lock.yaml"
          "*.svg"
          "*.img"
          "*.drawio"
        ];
      };
    };
  };
}
