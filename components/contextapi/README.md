# HWaaS ContextAPI

HWaaS context are domains of Isolation.
The ContextAPI is responsible to offer access to those
contexts and keep them isolated.

## Audit

Cargo audit is used for detecting vulnerabilities.

Updating the advisory-db is done by:
`nix flake lock --update-input advisory-db`

We support the generation of Gitlab quality report
reflecting vulnerabilities: `nix build .#audit`

:warn: Audit is not part of flake check output.
The reason is that the occurrence of vulnerability reports
and development in this repository are not linked.
Auditing must happen in relation to time and not to commits.
Otherwise, new reports impact the build result without
changes to our code.
The audit gitlab job is allowed to fail as we are not necessarily
able to fix all incidents ourselves but rely on crate authors.

## Debug via tokio-console

1. build a NixOS integration test of the context api which uses the test debug
   module.

   <!-- cspell:disable -->

   ```bash
   # this is one example test:
   nix build .#debug-remote-hands-aux-device-test

   # run the interactive test driver:
   ./result/bin/nixos-test-driver
   ```

   <!-- cspell:enable -->

1. in the spawned python console of the test-driver you have to
   start the VM which contains the ContextAPI:

   ```python
   start_all()
   ```

1. within the launched Qemu window perform the login to get shell access and
   simply enter:

   ```bash
   tokio-console
   ```

1. profit
