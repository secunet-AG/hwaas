// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use init_tracing_opentelemetry::resource::DetectResource;
use init_tracing_opentelemetry::{init_propagator, otlp};
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_sdk::trace::{Tracer, TracerProvider};
use std::cell::OnceCell;
use std::fs::File;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tower::BoxError;
use tracing::{Level, Subscriber, info};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt};

/// Build a new HWaaS tracing instance.
/// This builder has sane defaults for the HWaaS project.
///
/// Issuing the build will set up the actual [`tracing_subscriber`].
/// Once issued successful [`tracing`] could be used normally via e.g. its macros.
///
/// # Examples
/// ```
/// use hunt::HuntBuilder;
/// HuntBuilder::new()
///     .verbosity(2)
///     .append_filters(vec![
///         "crate_to_include",
///         "another_crate_to_include",
///     ])
///     .fallback_name(env!("CARGO_PKG_NAME"))
///     .fallback_version(env!("CARGO_PKG_VERSION"))
///     .build();
/// ```
#[derive(Debug, Default)]
pub struct HuntBuilder(Hunt);

impl HuntBuilder {
    /// Init a new Hunt builder with default Hunt values.
    pub fn new() -> Self {
        Self(Hunt::default())
    }

    /// Finalize builder and return an initialized Hunt instance.
    pub fn build(self) -> Hunt {
        self.0.setup_tracing();
        self.0
    }

    /// Enable OpenTelemetry exporting layer.
    ///
    /// Can be configured via env-vars.
    /// See this [documentation](https://github.com/davidB/axum-tracing-opentelemetry#configuration-based-on-environment-variable)
    /// for available options.
    ///
    /// ## Panics
    /// Requires a running tokio runtime - expect a panic elsewise
    pub fn enable_otel_layer(mut self) -> Self {
        self.0.otel_layer_enable = true;
        self
    }

    /// A list of target crates to trace.
    /// The filter for these crates is set to the verbosity level (see [`HuntBuilder::verbosity`])
    pub fn append_filters(mut self, mut targets: Vec<&'static str>) -> Self {
        self.0.filter_targets.append(&mut targets);
        self
    }

    /// Set the fallback name of the service if it is not determinable via env-vars
    ///
    /// e.g. use `env!("CARGO_PKG_NAME")` to set this.
    pub fn fallback_name(mut self, name: &'static str) -> Self {
        self.0.service_fallback_name = name;
        self
    }

    /// Set the fallback version of the service if it is not determinable via env-vars
    ///
    /// e.g. use `env!("CARGO_PKG_VERSION")` to set this.
    pub fn fallback_version(mut self, version: &'static str) -> Self {
        self.0.service_fallback_version = version;
        self
    }

    /// If some socket address is provided, tokio console is enabled.
    /// See `<https://github.com/tokio-rs/console>`
    pub fn tokio_console_address(mut self, addr: Option<SocketAddr>) -> Self {
        self.0.tokio_console_address = addr;
        self
    }

    /// Set verbosity level respected by all filters for all layers.
    /// If not set [`Level::WARN`] is used.
    pub fn verbosity(mut self, verbosity: u8) -> Self {
        let level = match verbosity {
            0 => Level::WARN,
            1 => Level::INFO,
            2 => Level::DEBUG,
            _ => Level::TRACE,
        };

        self.0.level = level;
        self
    }
}

/// Handle for a [`Hunt`] instance. On drop the shutdown of tracing subscribers is initiated.
#[derive(Debug)]
pub struct Hunt {
    level: Level,
    service_fallback_name: &'static str,
    service_fallback_version: &'static str,
    otel_layer_enable: bool,
    filter_targets: Vec<&'static str>,
    tokio_console_address: Option<SocketAddr>,
    trace_provider: OnceCell<TracerProvider>,
}

impl Default for Hunt {
    fn default() -> Self {
        Self {
            tokio_console_address: None,
            service_fallback_name: "NoServiceName",
            service_fallback_version: "NoServiceVersion",
            level: Level::WARN,
            otel_layer_enable: false,
            filter_targets: vec![],
            trace_provider: OnceCell::new(),
        }
    }
}

impl Hunt {
    fn setup_tracing(&self) {
        let mut filter = EnvFilter::from_default_env().add_directive("hunt:=info".parse().unwrap());

        // w/o this directives, traceparent headers are not correctly propagated
        if self.otel_layer_enable {
            filter = filter
                .add_directive("otel::tracing=trace".parse().unwrap())
                .add_directive("otel=debug".parse().unwrap());
        }

        for e in &self.filter_targets {
            let d = format!("{}={}", e, self.level)
                .parse()
                .expect("string should be a valid EnvFilter directive");
            filter = filter.add_directive(d);
        }

        let file = File::create("debug_log.json")
            .expect("it should be possible to create and open the log file");
        let debug_log = fmt::layer().with_writer(Arc::new(file)).json();

        let otel = if self.otel_layer_enable {
            Some(
                self.build_otel_layer()
                    .map_err(|e| println!("otel Error: {:?}", e))
                    .expect("the otel layer should have been initialized successfully"),
            )
        } else {
            None
        };

        // spawn the console server in the background,
        // returning a `Layer` but only if an address was provided
        let console_layer = self.tokio_console_address.map(|a| {
            console_subscriber::ConsoleLayer::builder()
                .retention(Duration::from_secs(20))
                .server_addr(a)
                .spawn()
        });

        tracing_subscriber::registry()
            .with(otel)
            .with(console_layer)
            .with(tracing_subscriber::fmt::layer())
            .with(debug_log)
            .with(filter)
            .init();

        info!("Log level is set to: {:?}", self.level);

        if let Some(a) = self.tokio_console_address {
            info!("tokio console enabled ({})", a);
        }

        if self.otel_layer_enable {
            info!("OpenTelemetry exporter running");
        }
    }

    fn build_otel_layer<S>(&self) -> Result<OpenTelemetryLayer<S, Tracer>, BoxError>
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        let otel_trace_provider = self.trace_provider.get_or_init(|| {
            let otel_rsrc = DetectResource::default()
                .with_fallback_service_name(self.service_fallback_name)
                .with_fallback_service_version(self.service_fallback_version)
                .build();
            otlp::init_tracerprovider(otel_rsrc, otlp::identity)
                .expect("Open Telemetry provider should be initialized")
        });

        init_propagator()?;
        Ok(tracing_opentelemetry::layer()
            .with_tracer(otel_trace_provider.tracer(self.service_fallback_name)))
    }
}

impl Drop for Hunt {
    fn drop(&mut self) {
        opentelemetry::global::shutdown_tracer_provider();
    }
}
