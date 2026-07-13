// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use remote_axum::run_axum_server_with_cleanup;
use remote_init::CliArgs;
use remote_serial::serial::serial_task::SerialTasks;
use remote_serial::{
    api,
    app_config::{AppConfig, SerialConfig},
    app_state::AppState,
    make_cancel_hook,
    serial::{self},
};
use std::collections::HashMap;

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
    let app_config = args.load_config::<AppConfig>();
    let serials: HashMap<String, SerialTasks> = app_config
        .serials
        .into_iter()
        .map(|(name, serial_config)| {
            // Pick desired serial type
            let SerialConfig { serial_type } =
                serde_json::from_value(serial_config.clone()).expect("serial_config");
            let serial = match &serial_type[..] {
                "stdio" => serial::stdio::new_with_json(serial_config).unwrap(),
                "tty" => serial::tty::new_with_json(serial_config).unwrap(),
                _ => panic!("Unknown serial_type {serial_type}"),
            };
            (name, serial)
        })
        .collect();

    // Initialize AppState with config
    let app_state = AppState::new(serials.clone());

    // Boot web server
    let app = api::get_router(app_state).await.expect("api::get_router");
    run_axum_server_with_cleanup(args.address(), app, hunt, make_cancel_hook(serials))
        .await
        .map_err(|_| 1)
}
