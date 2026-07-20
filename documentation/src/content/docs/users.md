# User Guide

Welcome to the user documentation for the HWaaS!

If you're here we assume that you already have access to a working HWaaS
instance to follow along. If this is not the case, please refer to our [maintainer documentation](./managing-an-instance).

There are multiple methods to interact with HWaaS, each tailored to different
needs. If you want...

- a HTTP/REST API for low-level interaction and maximum control, check out the
  [Context API](/api)
- a CLI-based solution with a higher abstraction for scripting or interactive
  use, check out the [User Tooling](/docs/users#user-tooling)
- a low-effort interface for casual or one-time usage, check out the [Web UI](/docs/users#web-ui)

## Context API

The Context API is a HTTP/REST API for programmatic interaction with HWaaS
instances. It's the core for all HWaaS operations and provides the basic
building blocks used by other means of interaction like the [User Tooling](/docs/users#user-tooling)
or [Web UI](/docs/users#web-ui).

Context API is fundamentally built around the concept of a _Context_, which is
an exclusive lease of one or more physical machines (also called Bare Metal
Resources, **BMR** for short) attached to HWaaS. For the duration of this
lease, the BMRs are reserved for use by the holder of the Context. Contexts
have a limited lifetime and will expire either after a preconfigured maximum
time, or when they are prematurely "returned" to HWaaS by the user.

Contexts are reserved from the _Inventory_, which is a complete list of all
registered BMRs. For every BMR, the Inventory tracks the BMRs current
reservation status and its individual properties. [Machine properties][ca0] are
used to distinguish individual BMRs and are configured by the HWaaS
administrator during BMR onboarding.

[ca0]: TODO(hartan): Link to `MachineProperties` rust docs

Once you have acquired a Context, you can configure the respective BMR within
the confines that the Context API provides. At the moment, the following
resources can be managed for BMRs inside a Context:

- Device Power
- (physical) Serial Interfaces for arbitrary I/O
- (virtual) USB drives for arbitrary I/O
- (virtual) Keyboard and Mouse inputs
- (virtual) Networking between two or more BMRs for arbitrary I/O

Each of these resources is bound to the Context that creates it. This means
that when a Context expires, all created resources are released and cleaned up.
Subsequent Contexts have to recreate all desired resources from scratch.

The HWaaS and Context API is only concerned with managing BMRs and associated
virtual resources. From this point onward, it's your responsibility to do
something useful with the reserved resources. You can find some inspiration in
the [Examples][?] and [Cookbook][?].

## User Tooling

The User Tooling is a collection of specialized Python modules to run tests and
benchmarks on HWaaS instances. It aims to simplify the onboarding of new
projects while allowing users to create complex tests and benchmarks.

User Tooling uses the [Context API](/api) and abstracts many common tasks into
convenient Python functions. It is fully integrated with the `nix` package
manager and _NixOS_ integration testing framework. When used with `nix` for
your project, User Tooling allows fully declaring the test environment (i.e.
the _Context_ from the [Context API](/api)) and the test to run within this
environment in a single location.

TODO(hartan): Can this be used interactively somehow?!

You can find usage examples of this in the [Cookbook][?].

## Web UI

The Web UI is an interactive Web UI to control the [Context API](/api) without
additional tooling. It is integrated into HWaaS deployments by default and aims
to simplify one-off interactions with the [Context API](/api).

In contrast to the [Context API](/api) and [User Tooling](/docs/users#user-tooling), the Web UI has
integrated Keyboard, Video and Mouse (**KVM**) support. This makes it the
preferred choice for interactive device exploration.

You can find usage examples of this in the [Cookbook][?].

## Examples

<!-- scope: provide fully-working and tested step-by-step guides to achieve a well-defined goal -->

This section of the manual provides practical and fully tested examples to
achieve certain tasks with the HWaaS. If your particular use case isn't listed
in any of the examples, we recommend you pick the most useful example and head
over to the [Cookbook](/docs/users#cookbook) for some tips and practical guidance.

TODO(hartan): I think each of these should be a fully standalone and commented
file that users can execute in a simple command or maybe two. Preferably these
are fully integrated with `nix` to include relevant dependencies etc.

### Booting A Firmware Image

#### Requirements

- _A fully working HWaaS instance_
- A PC to interact with the HWaaS
- One BMR registered with your HWaaS instance
- A Firmware Image to boot

#### Description

- Language for Context API: `bash` with `curl` and `jq`
- Language for User Tooling `nix` with `python`
- Language for Web UI: `playwright` or what it's called with `chrome`?

## Cookbook

### Two Machine HTTP Server
