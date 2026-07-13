// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use remote_auxiliary::{api, app_state::AppState};

#[tokio::main]
/// Generate the OpenAPI Spec for the `remote-auxiliary` service.
async fn main() -> Result<(), u8> {
    let state = AppState::default();
    let json = serde_json::to_value(api::get_api::<()>(state).await.unwrap()).unwrap();
    let json_str = serde_json::to_string_pretty(&json).unwrap();
    println!("{}", json_str);

    Ok(())
}
