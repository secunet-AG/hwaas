// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use remote_axum::run_axum_server;
use remote_init::CliArgs;
use remote_usb::{api, app_config::AppConfig, app_state::AppState};

#[tokio::main]
async fn main() -> Result<(), u8> {
    // Parse command-line arguments
    let args = CliArgs::parse();

    // Initialize logging
    let hunt = args
        .hunt()
        .append_filters(vec![
            "remote_init",
            "remote_axum",
            "remote_serial",
            env!("CARGO_PKG_NAME"),
        ])
        .fallback_name(env!("CARGO_PKG_NAME"))
        .fallback_version(env!("CARGO_PKG_VERSION"))
        .build();

    // Load config
    let config = args.load_config::<AppConfig>();
    // Initialize AppState with config
    let state = AppState::new(config.images_path);

    // Boot web server
    let app = api::get_router(state).await.expect("api::get_router");
    run_axum_server(args.address(), app, hunt)
        .await
        .map_err(|_| 1)
}
