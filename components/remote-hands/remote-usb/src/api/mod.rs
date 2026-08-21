// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

mod hid;
mod usb;

use aide::openapi::OpenApi;
use axum::{Json, Router};
#[cfg(feature = "usb-serial")]
use remote_serial::api::serial_router;
use serde_json::{Value, json};
use std::convert::Infallible;

use self::hid::hid_router;
use self::usb::usb_router;
use crate::app_state::UsbConfigurable;

pub use hid::{KeyboardReport, MouseReport};

/// Transform the given error into a Json value
fn json_error<E: std::fmt::Debug>(e: E) -> Json<Value> {
    Json(json!({"error": format!("{:?}", e)}))
}

/// Create the router for the `UsbAPI`
pub async fn get_router<S, T: UsbConfigurable>(state: T) -> Result<Router<S>, Infallible> {
    Ok(prepare_api_router(state).await?.0)
}

/// Build a `OpenAPI` based on the router implementation
pub async fn get_api<S, T: UsbConfigurable>(state: T) -> Result<OpenApi, Infallible> {
    Ok(prepare_api_router::<S, T>(state).await?.1)
}

/// Prepare router with all sub routes.
pub async fn prepare_api_router<S, T: UsbConfigurable>(
    state: T,
) -> Result<(Router<S>, OpenApi), Infallible> {
    let usb_router = usb_router();
    let hid_router = hid_router();

    let (router, api) = remote_axum::api_router(
        "remote-hands usb service",
        env!("CARGO_PKG_VERSION"),
        |router| async {
            let router = router.merge(usb_router).merge(hid_router);

            #[cfg(feature = "usb-serial")]
            let router = router.merge(serial_router());

            Ok::<_, Infallible>(router)
        },
    )
    .await?;

    let router = router.with_state(state);
    Ok((router, api))
}
