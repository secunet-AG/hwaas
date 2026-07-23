# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ inputs, ... }: {
  imports = [ inputs.git-hooks-nix.flakeModule ];
  perSystem = { config, pkgs, ... }: {
    pre-commit = {
      check.enable = true;
      inherit pkgs;
      settings.hooks = {
        treefmt = {
          enable = true;
          package = config.formatter;
        };
        statix = {
          enable = true;
          settings.ignore = [
            "user-tooling/"
          ];
        };
        deadnix.enable = true;
        reuse.enable = true;
      };
    };
  };
}
