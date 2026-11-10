# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
# SPDX-License-Identifier: Apache-2.0

{
  perSystem = { pkgs, ... }: {
    apps.fix-dependabot-pr = {
      type = "app";
      program = pkgs.writeShellApplication {
        name = "fix-dependabot-pr";
        text = builtins.readFile ../../bin/fix-dependabot-pr.sh;
        runtimeInputs = with pkgs; [
          gitMinimal
          coreutils
          curl
          jq
          bashNonInteractive
        ];
        inheritPath = false;
      };
      meta.description = "Manually check out and fix dependabot PRs from the GitHub repository.";
    };
  };
}
