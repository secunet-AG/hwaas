# Hunt

HWaaS universal notorious tracing (Hunt) is an opinionated library crate to
initialize tracing, tracing subscribers and subordinate logging features
(e.g. OpenTelemetry handling/processing).

## Usage

Hunt should be initialized as early as possible.

```rust
use hunt::HuntBuilder;
use clap::Parser;

#[derive(Parser, Debug)]
struct CliArgs {
    /// level of verbosity (could be used several times; e.g. '-vv')
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// further args
}

fn main() {
    // e.g. parse cli args via clap
    let args: CliArgs = CliArgs::parse();

    HuntBuilder::new()
        .set_verbosity(args.verbose)
        .append_filters(vec![
            "crate_to_include",
            "another_crate_to_include",
        ])
        .set_logfile("/path/to/logfile")
        .set_fallback_name(env!("CARGO_PKG_NAME"))
        .set_fallback_version(env!("CARGO_PKG_VERSION"))
        .build();

    // further code
}
```

### Enable OpenTelemetry export

[OpenTelemetry](https://opentelemetry.io/docs/what-is-opentelemetry/) (aka. otel)
is a standard way of exporting information form instrumented applications.
Instrumentation is done via the [tracing](https://crates.io/crates/tracing) crate.

To export otel data one has simply to enable the exporter during build of a
Hunt instance:

```rust
use hunt::HuntBuilder;

#[tokio::main]
async fn main() {
    HuntBuilder::new()
        .set_verbosity(args.verbose)
        .enable_otel_layer() // <- requires a running tokio async runtime
        // further builder args
        .build();

    // further code
}
```

Underneath Hunt utilizes the OpenTelemetry-SDK which can be configured through
\[environment variables}(<https://opentelemetry.io/docs/concepts/sdk-configuration/general-sdk-configuration/>).

### Make axum services otel-aware

An axum router can be wrapped via `hunt_axum_router` by needed middlewares,
to process needed headers.

```rust
use hunt::HuntBuilder;
use hunt::get_router;
use hunt::hunt_axum_router;

const ADDRESS: &str = "127.0.0.1:8080";

#[tokio::main]
async fn main() {
    HuntBuilder::new()
        // further builder args
        .build();

    let service = hunt_axum_router(get_router(inventory_data));

    Server::bind(ADDRESS)
        .serve(service)
        .await
        .expect("the service should never stop")

    // further code
}
```

### Forward tracing information

TBD

### Use tokio console

TBD

### Custom filters

Hunt allows to set global filter policies according to the environment variable
`[RUST_LOG](https://crates.io/crates/env_logger)`.
