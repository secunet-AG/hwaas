# User Tooling for Benchmarks

This repository contains tooling to simplify the creation of tests and
benchmarks for the HWaaS.

## Overview

The [user tooling](https://secunet-ag.github.io/hwaas/docs/users#user-tooling)
is meant to be the go to way to create tests and benchmarks for the HWaaS. The
goal is to make it easy to onboard new projects to run tests on the HWaaS.
Simple test and benchmark cases should be easy to create, while the user is
not prevented from building complex tests and benchmarks.

The main part of the user tooling is a Nix library wrapping the NixOS
integration tests. The wrapping library allows to create tests or benchmarks
that reserve machines in the HWaaS, connect to the HWaaS network and interact
with the machines. The user is able to specify the HWaaS machines, images to
boot and HWaaS networks via Nix, while the HWaaS Python library allows for
runtime interaction with the HWaaS machines.

## How to use

Have a look into the [examples folder](../nix/examples/)
to get an overview of how to use the tooling.
