# Architecture

The purpose of this document is to give a bird's eye view of the repository
to help maintainers know where to look for and place things.

We try to keep the details to a minimum in order to lower the chances of this
document getting out of sync with the code base itself.

## The main binaries

This is the repository for the Context API service and as such the
most important binary found here is the `contextapi` which starts
this service.

All other binaries exist to support the Context API or its users
in some way. The arguably most important of these binaries is
`machine-ops` which is a CLI tool for HWaaS maintainers to help
them with typical maintainer tasks such as for instance making
new hardware available to the Context API users without having
to restart the API.

## The context api crate

The `contextapi` is the crate where the binary that runs the Context API service
resides. If you are looking for a particular handler for a context api endpoint
then this crate is a good place to start. The rare exception is if the handler
has very few dependencies to other context api concepts (such as for example
contexts or machines).

The majority of tests for the context api service reside in `src/tests`
and are written in an "integration test" style, meaning we launch a
server per test and exercise the API through a HTTP client. This ensures
that we are testing the (public) API as users see it rather than internal
details that are subject to change.

## Database interaction

All types directly related to database functionality are placed in the
`db_interaction` crate. This also includes database migrations which
will automatically be placed there by the `diesel` cli tool as long
as it is called from the root directory.

Note that as of now database migrations are typically limited
to creating tables in the database. We typically populate the
database through other means such as by using the `machine-ops`
tool.

Functions that simplify database interactions should also be in `db_interaction`
unless they are rather service specific to the point that they require non
database related dependencies.

## The machine ops library

The `machine_ops_lib` crate exists primarily to assist the `machine-ops` CLI
tool, but is extracted as its own crate in order to assist testing the Context
API without necessarily needing to call the CLI tool from Rust to set up
necessary state.

## The context data structures crate

The `context_data_structures` crate contains (de)-serializable data structures
that users will directly consume and/or produce.

## Nix

Anything nix related apart from the flake in the root directory goes here.
This includes NixOS modules, integration tests and any Nix expressions
that are necessary to deploy the Context API and its related services.
