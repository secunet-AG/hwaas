# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

# This is a regression test to prevent concurrent access of the same
# socket/HID device for the `remote-usb` service.
{ testers, modules }:
let
  port = 8080;

  usbConfig.images_path = "/tmp";
in
testers.nixosTest {
  name = "remote-usb-hid-contention-test";

  nodes = {
    sut = { pkgs, ... }: {
      imports = [ modules.remote-usb ];

      services.remote-usb = {
        enable = true;
        inherit port;
        configFile = builtins.toFile "remote-usb.json" (builtins.toJSON usbConfig);
      };
      environment.systemPackages = with pkgs; [
        httpie
        websocat
      ];

      # Provide a mock usb device controller for remote-usb
      # `/dev/hidN` will then "exist" without real gadget hardware
      boot.kernelModules = [ "dummy_hcd" ];
    };
  };

  testScript = ''
    from time import sleep

    start_all()
    sut.wait_for_unit("remote-usb.service")

    usb_url = "http://localhost:${toString port}/usb"
    mouse_ws = "ws://127.0.0.1:${toString port}/usb/mouse/websocket"
    keyboard_ws = "ws://127.0.0.1:${toString port}/usb/keyboard/websocket"

    usb_functions = '[{"type":"mouse"},{"type":"keyboard"}]'
    sut.succeed(f"http --check-status PUT {usb_url} <<<'{usb_functions}'")

    # Connects, leaving the connection open and tagging it
    def open_conn(url, tag):
      sut.execute(f"""
        set -euo pipefail

        pidfile="/tmp/{tag}.pid"

        # Abort if connection for tag already present
        if [[ -f "/tmp/$pidfile" ]]; then
          echo "a connection for tag '{tag}' already exists" 1>&2
          exit 1
        fi

        websocat --no-close {url} < /dev/null &> /tmp/{tag}.log &
        pid="$!"
        disown -h "$pid"
        echo "$pid" > "$pidfile"
      """)

    def is_alive(tag):
      status, _ = sut.execute(f"kill -0 $(cat /tmp/{tag}.pid) 2>/dev/null")
      return status == 0

    def stop(tag):
      sut.execute(f"kill $(cat /tmp/{tag}.pid) 2>/dev/null; true")

    with subtest("a single connection is accepted and stays open"):
      open_conn(mouse_ws, "m1")
      sleep(1)
      assert is_alive("m1"), "the only mouse connection should not be rejected"
      stop("m1")
      sleep(1)

    with subtest("a second concurrent mouse connection is rejected"):
      open_conn(mouse_ws, "m1")
      sleep(1)
      assert is_alive("m1"), "first mouse connection should still be open"

      open_conn(mouse_ws, "m2")
      sleep(1)
      assert not is_alive("m2"), "second concurrent mouse connection must be closed by the server"
      assert is_alive("m1"), "first mouse connection must be unaffected by the rejected second one"

      stop("m1")
      sleep(1)

    with subtest("mouse becomes available again once the active connection closes"):
      open_conn(mouse_ws, "m3")
      sleep(1)
      assert is_alive("m3"), "mouse should accept a new connection after the previous one closed"
      stop("m3")

    with subtest("the same guard applies independently to the keyboard connection"):
      open_conn(keyboard_ws, "k1")
      sleep(1)
      assert is_alive("k1")

      open_conn(keyboard_ws, "k2")
      sleep(1)
      assert not is_alive("k2"), "second concurrent keyboard connection must be closed by the server"
      assert is_alive("k1")

      stop("k1")
      sleep(1)

    with subtest("mouse and keyboard guards do not interfere with each other"):
      open_conn(mouse_ws, "m4")
      open_conn(keyboard_ws, "k3")
      sleep(1)
      assert is_alive("m4"), "an active keyboard connection must not block the mouse"
      assert is_alive("k3"), "an active mouse connection must not block the keyboard"
      stop("m4")
      stop("k3")
  '';
}
