# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ testers
, hunt
,
}:
testers.nixosTest {
  name = "rust-log";

  nodes = {
    sut = {
      environment.systemPackages = [ hunt ];
    };
  };

  testScript = ''
    import json

    def run(rust_log=""):
      sut.succeed("rm debug_log.json || true")
      sut.succeed(f"RUST_LOG=\"{rust_log}\" hunt-test-app")
      log = sut.succeed("cat debug_log.json | tail -n1")
      json_log = json.loads(log)

      # delete the timestamp to compare more easily
      del json_log["timestamp"]

      return json_log

    start_all()

    with subtest("Without env"):
      expected = json.loads('{"level":"INFO","fields":{"message":"Hello World"},"target":"hunt_test_app::enabled_log"}')
      assert run() == expected, "log data should match"

    with subtest("With env"):
      expected = json.loads('{"level":"INFO","fields":{"message":"foo bar"},"target":"hunt_test_app::not_enabled_log"}')
      assert run(rust_log="hunt_test_app::not_enabled_log") == expected, "log data should match"

  '';
}
