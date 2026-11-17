# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ ... }:

{
  /**
    Create a HWaaS NixOS test.

    This is the main entry point for the HWaaS user tooling.

    # Running HWaaS tests

    Since HWaaS tests *must* communicate with a HWaaS instance, they cannot be run from inside the
    regular `nix` sandbox. The sandbox prevents network communication and makes it impossible to
    reserve Contexts, for example. The following subsections present different ways to run HWaaS
    tests, from most to least recommended.

    ## Build and run the test manually

    Build the `.driver` attribute of the resulting derivation and run the test manually. In
    `nix` you would write something like:

    ```nix
    outputs = { nixpkgs, hwaas, ... }: {
      packages.x86_64-linux.my-hwaas-test = hwaas.lib.hwaasTest nixpkgs {
        # ...
      };
    }
    ```

    Then run the following commands:

    ```bash
    nix build .#my-hwaas-test.driver
    ./result/bin/nixos-test-driver
    ```

    This approach is great for debugging, since the `nixos-test-driver` executable accepts
    additional arguments that can help uncover issues in your test.

    ## Build and run the test as flake app

    This approach complements the "Build and run the test manually" instructions by integrating test
    execution more tightly into the nix flake. In `nix` you would write something like:

    ```nix
    apps.x86_64-linux.execute-my-hwaas-test = {
      type = "app";
      program = "${config.packages.x86_64-linux.my-hwaas-test.driver}/bin/nixos-test-driver";
    };
    ```

    Then run the following command:

    ```bash
    nix run .#execute-my-hwaas-test
    ```

    This approach saves you from executing multiple commands yourself. Since it reuses the test that
    is exposed as package in the flake, you can still build and run the package manually for debugging
    if desired.

    ## Build the test without the NixOS sandbox

    This approach disables the `nix` sandbox for the duration of test. Building upon the "Build and
    run the test manually" section above, you run the following command:

    ```bash
    # Note the absence of the `.driver` suffix
    nix build --no-sandbox .#my-hwaas-test
    ```

    # Inputs

    `pkgs`
    : An instance of `nixpkgs` to draw packages and utility functions from.

    `test`
    : A description of the HWaaS test, similar to a plain NixOS test.
      Compared to regular NixOS tests, this attribute set must define additional options.

    # Type

    ```text
    AttrSet -> AttrSet -> derivation
    ```
  */
  flake.lib.hwaasTest =
    pkgs: import ../../../user-tooling/packages/hwaas-integration-test/default.nix { inherit pkgs; };
}
