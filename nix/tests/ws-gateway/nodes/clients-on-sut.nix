# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ serverIp
, serverPort
, sharedModule
, modules
,
}:
{ number
, net
, sutIp
,
}:
builtins.listToAttrs (
  builtins.genList
    (x: {
      name = "client" + builtins.toString (x + 1) + "Net" + (builtins.toString net);
      value =
        { ... }:
        {
          imports = [
            sharedModule
            ./client-template.nix
            modules.test-debug-module
            modules.ws-client-module
          ];

          services.simHwaasClient = {
            enable = true;
            inherit
              net
              serverIp
              serverPort
              sutIp
              ;
            ip = "11.11.11." + builtins.toString (x + 12);
            ipTap = "192.168.1." + builtins.toString (x + 22);
          };
        };
    })
    number
)
