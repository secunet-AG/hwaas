# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ inputs, ... }: {
  perSystem = { pkgs, system, ... }: {
    _module.args.pkgs = import inputs.nixpkgs {
      inherit system;
      overlays = [
        (import inputs.rust-overlay)
        (_final: prev: {
          python3 = prev.python3.override {
            packageOverrides = _pFinal: pPrev: {
              plantuml-markdown = pPrev.plantuml-markdown.override {
                # When the display breaks in runtime check if the c4-lib or sprites
                # lib that is pinned in the nixpkgs package was changed
                plantuml = pkgs.plantuml-c4;
              };
            };
          };
          # Pach needed until https://github.com/renovatebot/renovate/pull/33991 made its way to the
          # nix package of renovate
          renovate = prev.renovate.overrideAttrs {
            patches = prev.patches or [ ] ++ [
              (pkgs.fetchpatch {
                url = "https://github.com/renovatebot/renovate/pull/33991.diff";
                hash = "sha256-3sN9a0ydk/ZLzPGVkir3mnM3f70dS3kyqezwBg/WWkQ=";
              })
            ];
          };
        })
      ];
    };

  };
}
