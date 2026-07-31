# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ testers
, modules
,
}:
let
  port = 8080;
  serialPath = "/dev/ttyS0";

  mkTest =
    name: serialConfig: testScript:
    testers.nixosTest {
      inherit name;

      nodes = {
        sut =
          { pkgs, ... }:
          {
            imports = [ modules.remote-serial ];

            services.remote-serial = {
              enable = true;
              inherit port;
              configFile = builtins.toFile "remote-serial.json" (builtins.toJSON serialConfig);
            };

            environment.systemPackages = with pkgs; [
              httpie
              websocat
              util-linux
            ];
          };
      };

      inherit testScript;
    };

  serialConfigDefault = {
    serials = {
      test = {
        type = "tty";
        path = serialPath;
      };
    };
  };

  serialConfigCustom = {
    serials = {
      test = {
        type = "tty";
        path = serialPath;
        baud_rate = 9600;
        char_size = 5;
        stop_bits = 2;
        parity = "odd";
      };
    };
  };

in
{
  remote-hands-remote-serial-udev-default-test =
    mkTest "remote-serial-udev-default-test" serialConfigDefault
      ''
        with subtest("udev rule with serial defaults"):
          tty_rule = sut.succeed("grep 'tty' /etc/udev/rules.d/99-local.rules")
          assert "${serialPath} 115200 cs8 -cstopb -parenb raw" in tty_rule, f"Expected serial defaults, got '{tty_rule}'"
      '';

  remote-hands-remote-serial-udev-custom-test =
    mkTest "remote-serial-udev-custom-test" serialConfigCustom
      ''
        with subtest("udev rule with custom serial config"):
          tty_rule = sut.succeed("grep 'tty' /etc/udev/rules.d/99-local.rules")
          assert "${serialPath} 9600 cs5 cstopb parenb parodd raw" in tty_rule, f"Expected custom serial, got '{tty_rule}'"
      '';
}
