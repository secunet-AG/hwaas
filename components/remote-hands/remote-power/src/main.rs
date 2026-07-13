// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use remote_axum::run_axum_server;
use remote_init::CliArgs;
use remote_power::{api, app_config::AppConfig, power, AppState};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), u8> {
    // Parse command-line arguments
    let args = CliArgs::parse();

    // Initialize logging
    let hunt = args
        .hunt()
        .append_filters(vec!["remote_init", "remote_axum", env!("CARGO_PKG_NAME")])
        .fallback_name(env!("CARGO_PKG_NAME"))
        .fallback_version(env!("CARGO_PKG_VERSION"))
        .build();

    // Load config
    let config = args.load_config::<serde_json::Value>();
    let app_config = serde_json::from_value::<AppConfig>(config.clone()).expect("app_config");
    let controls = app_config
        .controls
        .into_iter()
        .map(|(name, control_config)| (name, Arc::new(Mutex::new(power::new(control_config)))))
        .collect();

    // Initialize AppState
    let state = AppState {
        controls: Arc::new(controls),
    };

    // Boot web server
    let app = api::get_router(state).await.expect("api::get_router");
    run_axum_server(args.address(), app, hunt)
        .await
        .map_err(|_| 1)
}
