// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use remote_auxiliary::{api, app_config::AppConfig, app_state::AppState, auxiliary};
use remote_axum::run_axum_server;
use remote_init::CliArgs;

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
    let aux_devices = app_config
        .aux_devices
        .into_iter()
        .map(|(name, aux_config)| (name, auxiliary::new(aux_config)))
        .collect();

    // Initialize AppState with config
    let state = AppState::new(aux_devices);

    // Boot web server
    let app = api::get_router(state).await.expect("api::get_router");
    run_axum_server(args.address(), app, hunt)
        .await
        .map_err(|_| 1)
}
