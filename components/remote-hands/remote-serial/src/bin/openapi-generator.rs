// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use remote_serial::{api, app_state::AppState};

#[tokio::main]
/// Generate the OpenAPI Spec for the `remote-serial` service.
async fn main() -> Result<(), u8> {
    // Wrap serial statefully
    let app_state = AppState::default();
    let json = serde_json::to_value(api::get_api::<()>(app_state).await.unwrap()).unwrap();
    let json_str = serde_json::to_string_pretty(&json).unwrap();
    println!("{}", json_str);

    Ok(())
}
