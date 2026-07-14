# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

_: {
  perSystem =
    { config, pkgs, ... }:
    {
      packages = rec {
        net-ctrl-oas = pkgs.callPackage ./net-ctrl-oas.nix {
          inherit (config.packages) net-ctrl;
        };
        remote-hands-oas = pkgs.callPackage ./remote-hands-oas.nix {
          inherit (config.packages)
            remote-serial
            remote-power
            remote-usb
            remote-auxiliary
            ;
        };
        net-ctrl-client = pkgs.callPackage ./net-ctrl-client.nix {
          net-ctrl-openapi = ../../expected-oas/net-ctrl.openapi.json;

          # We could have used this instead, but unfortunately there are some evaluation errors
          # due to our virtual manifest for all crates. Building the net-ctrl package for obtaining
          # the OAS generator binary requires already that the generated net-ctrl-client is visible
          # for the context api.
          #net-ctrl-openapi = config.packages.net-ctrl-oas;
        };
        contextapi-version = pkgs.callPackage ./contextapi-version.nix {
          inherit (config.packages) contextapi;
        };
        contextapi-oas = pkgs.callPackage ./contextapi-oas.nix {
          inherit (config.packages) contextapi;
        };
        hwaas-oas = pkgs.callPackage ./hwaas-oas.nix {
          inherit (config.packages) contextapi;
          inherit remote-hands-oas;
        };
        http-sim = pkgs.callPackage ./http-sim/package.nix { };
        vue-client = pkgs.callPackage ./vue-client.nix {
          vueSrc = ../../vue-client;
        };
        documentation = pkgs.callPackage ./documentation.nix {
          documentationSrc = ../../documentation;
          inherit hwaas-oas;
        };
      };
    };
}
