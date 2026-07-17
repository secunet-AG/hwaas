# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ inputs, ... }:
let
  user-tooling-per-system = system: pkgs: import "${inputs.user-tooling-src}/default.nix" {
    inherit system pkgs;
    pre-commit-hooks = inputs.git-hooks-nix.lib.${system};
  };
  # Map non-flake attributes to flattened flake attributes
  nonFlakeToFlakeAttrs =
    lib: attrs:
    lib.mapAttrs'
      (name: value:
      lib.nameValuePair "user-tooling-${name}" value
      )
      attrs;
in
{
  perSystem =
    { pkgs, system, lib, ... }:
    let
      user-tooling = user-tooling-per-system system pkgs;
    in
    {
      # Skip `hwaasTest` since it is not a buildable package
      packages = nonFlakeToFlakeAttrs lib (lib.filterAttrs (name: _: name != "hwaasTest") user-tooling.packages);
      checks = nonFlakeToFlakeAttrs lib user-tooling.checks;
      # Skipping examples here as well (for now), since we need to do a bit more to make them executable again
    };

  flake.nixosModules =
    let
      # Modules normally should not require a concrete pkgs/system just to
      # be exported. This import is only safe if evaluating nixosModules does
      # not force the package/check portions of default.nix.
      system = "x86_64-linux";
      pkgs = import inputs.nixpkgs {
        inherit system;
      };
      user-tooling = user-tooling-per-system system pkgs;
    in
    nonFlakeToFlakeAttrs pkgs.lib user-tooling.nixosModules;
}
