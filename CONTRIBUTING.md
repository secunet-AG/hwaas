# Contribution Guidelines

## Welcome

Thank you for your interest in contributing to the HWaaS.

We have a number of shared guidelines for any incoming contributions, that help us audit their quality
and determine if they are a good fit for the HWaaS ecosystem.

## Ground Rules

### Code of Conduct

Regardless of what or how you want to contribute, we expect all contributors to adhere to the [Berlin Code of Conduct](https://berlincodeofconduct.org/en).

### Issues

Issues are the strongest place to propose a new feature, change in functionality, or report a bug.

We propose the following guidelines for creating issues:

- Search the issues before proposing a feature, to ensure that it is not already under discussion.
- Please use our issue template if you have all of the required content. Otherwise, a more minimal issue is fine.
- It may be wise to hold off on large features or functionality until maintainers have decided that this is desired.

### Pull Requests

All PRs are expected to pass the checks in the repository.

This includes unit tests, integration, e2e testing, formatting, lints, etc. If your PR cannot pass these checks, it will not be taken seriously.

## Environment Setup

All development for the HWaaS project is expected to run and be built using Nix. In other words,
you must have a working [`nix` installation][d1].

> [!important]
> Within nix, the HWaaS project heavily utilizes the [experimental `flakes`
> feature][d2]. All commands in this documentation will assume you _have not_
> enabled this feature by default as is the case after a fresh installation. If
> you did enable this feature system-wide, you can run all nix commands without
> the `--extra-experimental-features "nix-command flakes"` CLI arguments.

To verify this worked, run the following command:

```bash
nix --extra-experimental-features "nix-command flakes" flake check --no-build
```

You can then manage your development environment like so

```bash
nix develop .
# Alternatively, allow direnv to manage it for you
direnv --allow
```

[d0]: https://nix.dev/
[d1]: https://nix.dev/install-nix
[d2]: https://nix.dev/manual/nix/2.34/development/experimental-features#xp-feature-flakes

## Best practices

Our project has adopted the following best practices for contributing:

- [REUSE.software][e0] for licensing and copyright information
- [Keep a Changelog][e1] for communicating changes between releases

[e0]: https://reuse.software
[e1]: https://keepachangelog.com/en/2.0.0/

## Governance

### Core & Plugin Philosophy

HWaaS is a shared platform designed to serve diverse HIL testing interests. To prevent fragmentation while encouraging innovation, we adhere to a **Component** architecture.

- **The Core:** Changes such as central orchestration logic, API contracts, breaking changes for deployment, or changes to the security model require broad maintainer consensus and must remain general-purpose.
- **Maintenance**: Non-breaking changes to dependencies, small hot-fixes, documentation updates, etc.
- **Other Requirements:** Significant features, niche hardware integrations, or specialized deployment logic should be implemented as **plugins, components, modules, or external deployment layers.** If a proposed change is highly specific to one maintainer’s use case but risks destabilizing or complicating the shared platform, the default path is implementation as an opt-in module rather than a core integration or breaking change for other platforms.

### Conflict Resolution & "Soft Forks"

We recognize that maintainers will have diverging priorities. In the event of a fundamental disagreement on the direction of a feature:

**Isolation:** Can the feature exist as a non-breaking extension? If yes, it is merged as a module or component.
**Experimental Flags:** Large changes may be merged behind feature flags to allow real-world testing without forced adoption.
**Upstream Stability:** No maintainer may push "instant deployment" c---hanges that alter the baseline behavior of the HWaaS ecosystem for others without a formal RFC and transition period.

### Decision Making & Review Cycles

To ensure long-term stability, we divide our development velocity into two streams:

#### 1. Core Architectural Changes

Major changes to the Context API contract, User Tooling, or any change that could break existing CI/CD pipelines require a **Formal RFC Process**.

- **Discussion Window:** All RFCs for core changes must be open for discussion for a **minimum of 14 days**.
- **Leave Awareness:** We recognize that long stretches of leave are common. If a core maintainer or critical stakeholder is on planned leave, the discussion window must be extended accordingly. **No architectural decision is finalized until all key stakeholders have had a fair opportunity to review.** If the maintainer is looking for extended leave past two weeks, they need to delegate their responsibility to another maintainer or team member.
- **Goal:** We aim to find consensus amongst maintainers and users rather than alienating any specific teams or maintainers.

#### 2. Hotfixes and Maintenance

Bug fixes, documentation improvements, and trivial patches do not require an RFC.

- **Aggressive Merging:** We encourage the aggressive merging of PRs that fix broken CI, security vulnerabilities, or bugs that impact current usability, provided they pass all automated checks.
- **Delegated Authority:** Any two maintainers can approve a hotfix. If a maintainer is unavailable due to leave, we empower the remaining active team to ship fixes to keep the project moving.

## Licensing

All contributions to this repository will fall under the license this project was distributed with.

Unless noted otherwise, this project and the source files inside this repository are licensed under the terms of the Apache License in Version 2.0.
Refer to REUSE.toml for exceptions to licensing for individual files and NOTICES.md for additional notices on some third-party applications.
