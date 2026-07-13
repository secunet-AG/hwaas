# HWaaS Websocket Gateway (PoC)

This application is supposed to transfer L2-Network Traffic via a
WebSocket connection.
This WS connection is established between the server and the client
part (corresponding to the websocket role).
Both parts forward traffic to the Linux kernel networking Stack via `AF_PACKET`.

## Build and Test

If there is a Minimum Supported Rust Version (MSRV) 1.60 installed you could
build this crate via cargo.

The packaging is done via nix. The contained flake exposes NixOS modules and
specifies NixOS integration tests.
The binaries are hence buildable via `nix build` and are located afterwards
within `./result`.
During build the unit tests run automatically (using `cargo test`).

The VM based integrations tests could be run via `nix flake check`.
This will build and run all test scenarios (see next section).

## Test Scenarios

| scenario         | description                                    |
| ---------------- | ---------------------------------------------- |
| many-vlans       | setup many vlans (>200) on a interface         |
| static-proxy     | one client connects to one SUT (ping+iperf)    |
| many-clients     | many client connects to one SUT (ping)         |
| clients-and-suts | many pairs of clients connects to a SUT (ping) |

If you want to run a particular scenario you can do so by running the following
command while replacing `<scenaro>`:

```shell
nix build .#checks.x86_64-linux.integration-test-<scenario>
```

## Manually debug scenarios

:warn: The assumption is you run this on a graphical desktop environment.

Interactively debugging a scenario is possible by running the following command
while replacing `<scenaro>`:

```shell
nix build .#checks.x86_64-linux.debug-<scenario> && ./result/bin/nixos-test-driver
```

Now you should see a python prompt where several functions and objects
exists already:

- `start_all()` will start all VMs and you see them via qemu-monitor
- `test_script()` run the defined test script
- `machine.start()` start a single VM (with name `machine`; available names depend on scenario) and display it
- There are more things you could do on machines: see [NixOS integration tests manual](https://nixos.org/manual/nixos/stable/index.html#ssec-machine-objects)
