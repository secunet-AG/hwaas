# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

_: {
  perSystem = { config, pkgs, ... }: {
    checks = {
      vue-client-licenses-check = pkgs.callPackage ./vue-client-licenses.nix {
        vueClient = config.packages.vue-client;
        noticeFile = ../../NOTICE.md;
      };
    };
  };
}
