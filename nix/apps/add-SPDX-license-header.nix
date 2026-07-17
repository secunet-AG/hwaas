# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

_: {
  perSystem =
    { pkgs, ... }:
    let
      # Auto annotate all files with the listed file types with SPDX headers.
      # Ignore stuff under `net_ctrl_client` since it is auto generated.
      # Look at `REUSE.toml` to see handling for that and other files.
      script = pkgs.writeShellScriptBin "add-SPDX-license-header" ''
        git ls-files \
          '*.js' \
          '*.py' \
          '*.rs' \
          '*.sh' \
          '*.ts' \
          '*.css' \
          '*.nix' \
          '*.sql' \
          '*.vue' \
          '*.html' \
          '*.envrc' \
          '*.gitignore' \
          '*.editorconfig' \
          '*.gitattributes' \
          | grep -v '^components/contextapi/net_ctrl_client/' \
          | xargs ${pkgs.reuse}/bin/reuse annotate \
            --copyright 'secunet Security Networks AG <https://www.secunet.com>' \
            --license 'Apache-2.0' \
            --year 2026 \
            --copyright-prefix spdx-string \
            --skip-unrecognised
      '';
    in
    {
      apps.add-SPDX-license-header = {
        type = "app";
        program = "${script}/bin/add-SPDX-license-header";
      };
    };
}
