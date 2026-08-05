# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

# deadnix: skip
perSystem@{ config, ... }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.services.remote-serial;
  username = "remote-serial";
  group = "dialout";
  runDir = "/run/remote-serial";

  parsed = builtins.fromJSON (builtins.readFile cfg.configFile);
  ttySerials =
    builtins.filter (s: s.type == "tty") # only look at tty serials
      # get rid of user name for serial, only keep configs
      (lib.mapAttrsToList (_name: config: config) parsed.serials);
  # Add defaults and override them with parameters from user config (if given).
  # Tests are under ../tests/remote-hands/remote-serial-udev.nix
  serials =
    with cfg.serialConfig;
    map (
      serial:
      {
        inherit
          baud_rate
          char_size
          stop_bits
          parity
          ;
      }
      // serial
    ) ttySerials;

  # turn serial entry into a udev rule string
  mkUdevRule = serial: ''
    SUBSYSTEM=="tty", SUBSYSTEMS=="usb", GROUP="${group}", KERNEL=="${builtins.baseNameOf serial.path}", RUN+="${sttyCommand serial}"
  '';
  # turn serial entry into an stty command
  sttyCommand =
    with cfg.serialConfig;
    serial:
    let
      # baud rate
      baudRate = toString serial.baud_rate;
      # char size (cs5..cs8)
      charSize = "cs${toString serial.char_size}";
      # stop bits: "cstopb" means 2, "−cstopb" means 1
      stopBits = if serial.stop_bits == 2 then "cstopb" else "-cstopb";
      # parity flags
      parity =
        if serial.parity == "odd" then
          "parenb parodd"
        else if serial.parity == "even" then
          "parenb -parodd"
        else
          "-parenb"; # none
    in
    "${pkgs.coreutils}/bin/stty -F ${serial.path} ${baudRate} ${charSize} ${stopBits} ${parity} raw";
in
{
  options.services.remote-serial = {
    enable = lib.mkEnableOption "Serial Peripheral API";

    # Serial config. Defaults chosen from serial2_tokio's `set_raw()` (if existing)
    # to be equal with what we do in Rust.
    serialConfig = lib.mkOption {
      type = lib.types.submodule {
        options = {
          baud_rate = lib.mkOption {
            type = lib.types.int;
            description = "Baudrate value. No value verification for now. Use with care.";
            default = 115200;
          };
          char_size = lib.mkOption {
            type = lib.types.enum [
              5
              6
              7
              8
            ];
            description = "char size: use x-bit binary communication";
            default = 8;
          };
          stop_bits = lib.mkOption {
            type = lib.types.enum [
              1
              2
            ];
            description = "Number of stop bits.";
            default = 1;
          };
          parity = lib.mkOption {
            type = lib.types.enum [
              "odd"
              "even"
              "none"
            ];
            description = "Parity checks";
            default = "none";
          };
        };
      };
      default = { };
      description = "Serial config defining baud rate, char size, stop bits and parity values.";
    };

    package = lib.mkOption {
      default = perSystem.config.packages.remote-serial;
      type = lib.types.package;
      description = ''
        The remote-serial package to use. This defaults to the release version.
      '';
    };

    address = lib.mkOption {
      type = lib.types.str;
      default = "0.0.0.0";
      example = "127.0.0.1";
      description = "The IP address to serve the remote-serial under.";
    };

    port = lib.mkOption {
      type = lib.types.port;
      default = 8080;
      example = 80;
      description = "The port used to serve the remote-serial.";
    };

    configFile = lib.mkOption {
      type = lib.types.str;
      example = "/user/config.json";
      description = "The path to the file containing the configuration in JSON.";
    };

    otelEndpoint = lib.mkOption {
      type = lib.types.str;
      default = "http://127.0.0.1:4317";
      description = "URL of the OpenTelemetry collector to send traces in otlp format";
    };

    consoleAddress = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      example = "127.0.0.1:6669";
      description = "Start tokio console if a socket address is provided.";
    };

    openFirewall = lib.mkEnableOption "opening the firewall";
  };

  config = lib.mkIf cfg.enable {

    users.users."${username}" = {
      isSystemUser = true;
      inherit group;
      description = "remote-serial";
    };

    # launch stty commands per serial as soon as udev sees the device pop up
    services.udev.extraRules = lib.strings.concatMapStrings mkUdevRule serials;

    systemd.services.remote-serial =
      let
        tokioConsole = lib.strings.optionalString (
          !builtins.isNull cfg.consoleAddress
        ) "--tokio-console-address ${cfg.consoleAddress}";
      in
      {
        description = "HWaaS remote-serial";
        wantedBy = [ "multi-user.target" ];
        after = [ "network-online.target" ];
        wants = [ "network-online.target" ];
        environment = {
          OTEL_SERVICE_NAME = "remote-serial";
          OTEL_EXPORTER_OTLP_ENDPOINT = cfg.otelEndpoint;
          OTEL_EXPORTER_OTLP_PROTOCOL = "grpc";
          OTEL_TRACES_SAMPLER = "always_on";
          OTEL_LOG_LEVEL = "info";
        };
        path = with pkgs; [ bash ];
        serviceConfig = {
          Type = "notify";
          User = "${username}";
          ExecStart = ''
            ${cfg.package}/bin/remote-serial \
                        -vv \
                        ${tokioConsole} \
                        --address ${cfg.address} \
                        --port ${toString cfg.port} \
                        --config-file ${cfg.configFile}
          '';
          WorkingDirectory = "${runDir}";
          TimeoutStartSec = "45min";
        }
        // lib.optionalAttrs (cfg.port <= 1024) {
          # For ports below 1024 a special capability is needed additionaly (CAP_NET_BIND_SERVICE)
          CapabilityBoundingSet = "CAP_NET_BIND_SERVICE";
          AmbientCapabilities = "CAP_NET_BIND_SERVICE";
        };
      };

    systemd.tmpfiles.rules = [ "d ${runDir} 775 ${username} ${group}" ];

    networking.firewall.allowedTCPPorts = lib.optional cfg.openFirewall cfg.port;
  };
}
