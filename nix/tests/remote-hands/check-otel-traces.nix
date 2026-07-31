# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ testers
, modules
,
}:
let
  powerPort = 8080;
  serialPort = 8081;
  auxPort = 8082;
  usbPort = 8083;

  powerConfig.controls = {
    custom = {
      type = "custom";
      config = {
        on = "echo on";
        off = "echo off";
        reset = "echo reset";
        query = "echo off";
      };
    };
  };

  serialConfig.serials = {
    test = {
      type = "stdio";
      command = "cat";
    };
  };

  auxiliaryConfig.devices = {
    generic = {
      config = {
        id = "generic";
        url = "http://localhost:serialPort";
        cmd = "echo test";
      };
    };
  };

  usbConfig = {
    images_path = "/tmp";
  };

in
testers.nixosTest {
  name = "otel-traces-test";

  nodes = {
    sut =
      { pkgs, ... }:
      {

        imports = [
          modules.remote-power
          modules.remote-serial
          modules.remote-usb
          modules.remote-auxiliary
          modules.test-otel-collector
        ];

        systemd.services = {
          remote-power = {
            requires = [ "otel-collector.service" ];
            after = [ "otel-collector.service" ];
          };
          remote-serial = {
            requires = [ "otel-collector.service" ];
            after = [ "otel-collector.service" ];
          };
          remote-auxiliary = {
            requires = [ "otel-collector.service" ];
            after = [ "otel-collector.service" ];
          };
          remote-usb = {
            requires = [ "otel-collector.service" ];
            after = [ "otel-collector.service" ];
          };
        };
        services = {
          otelCollector.enable = true;
          remote-power = {
            enable = true;
            port = powerPort;
            configFile = builtins.toFile "remote-power.json" (builtins.toJSON powerConfig);
          };
          remote-serial = {
            enable = true;
            port = serialPort;
            configFile = builtins.toFile "remote-serial.json" (builtins.toJSON serialConfig);
          };
          remote-auxiliary = {
            enable = true;
            port = auxPort;
            configFile = builtins.toFile "remote-auxiliary.json" (builtins.toJSON auxiliaryConfig);
          };
          remote-usb = {
            enable = true;
            port = usbPort;
            configFile = builtins.toFile "remote-usb.json" (builtins.toJSON usbConfig);
          };
        };
        environment.systemPackages = with pkgs; [
          httpie
          util-linux
        ];

        # Provide a dummy usb device controller (UDC) for remote-usb.
        boot.kernelModules = [ "dummy_hcd" ];
      };
  };

  # This test checks whether all micro-services emit correct Open-Telemetry
  # traces for API requests.
  testScript = ''
    start_all()

    # traceparent header example according to https://www.w3.org/TR/trace-context/#trace-context-http-headers-format
    TRACE_ID = "1a687ed3bd263ba29d721453171b3f3a"
    TRACE_PARENT_HEADER = f"traceparent:00-{TRACE_ID}-004eb04f8b67a8e4-01"

    power_url = "http://localhost:${toString powerPort}/power"
    serial_url = "http://localhost:${toString serialPort}/serial/test"
    aux_url = "http://localhost:${toString auxPort}/auxiliaries"
    usb_url = "http://localhost:${toString usbPort}/usb"

    def test_traces(service, url):
      # wait for service start
      sut.wait_for_unit(f"{service}.service")
      # make a request that gets traced via OTEL
      responseHeaders = sut.succeed(f"http --headers GET {url} {TRACE_PARENT_HEADER}")
      # the header must contain the same ID, other parts of the header may change
      assert TRACE_ID in responseHeaders, f"Expected '{TRACE_ID}' to be contained in '{responseHeaders}'"
      # check that the collector received a trace about the submitted request
      sut.wait_until_succeeds(f"journalctl -u otel-collector -g {service}", timeout = 10)

    test_traces("remote-power", power_url)
    test_traces("remote-serial", serial_url)
    test_traces("remote-auxiliary", aux_url)
    test_traces("remote-usb", usb_url)
  '';
}
