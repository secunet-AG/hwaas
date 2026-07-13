// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use axum::async_trait;
use hidg::{Button, Key, Modifiers};
use remote_usb::api;
use remote_usb::app_state::UsbConfigurable;
use remote_usb::usb_config::{UsbConfig, UsbFunctionInfo};

use remote_serial::api::HasSerial;
use remote_serial::serial::serial_state::SerialState;

#[derive(Clone)]
/// "Empty" dummy state for OpenAPI generator.
/// Used as a workaround to not need to configure an actual AppState,
/// since it depends on real hardware (`default_udc()` fails here)
pub struct DummyState;

#[async_trait]
/// Empty implementation of UsbConfigurable functions for DummyState.
/// Panic so that they don't get used accidentally.
impl UsbConfigurable for DummyState {
    async fn configure(&self, _usb_config: UsbConfig) -> Result<(), std::io::Error> {
        panic!("Should not be called by OpenAPI generator.");
    }
    async fn deconfigure(&self) -> Result<(), std::io::Error> {
        panic!("Should not be called by OpenAPI generator.");
    }
    async fn get_function_infos(&self) -> Option<Vec<UsbFunctionInfo>> {
        panic!("Should not be called by OpenAPI generator.");
    }
    async fn use_keyboard(
        &self,
        _modifier: Option<Modifiers>,
        _keys: Vec<Key>,
        _press: bool,
        _release: bool,
    ) -> Result<(), std::io::Error> {
        panic!("Should not be called by OpenAPI generator.");
    }
    async fn use_mouse(
        &self,
        _buttons: Vec<Button>,
        _pointer: (i16, i16),
        _wheel: i8,
    ) -> Result<(), std::io::Error> {
        panic!("Should not be called by OpenAPI generator.");
    }
}

#[async_trait]
/// Empty implementation of HasSerial functions for DummyState.
/// Panic so that they don't get used accidentally.
impl HasSerial for DummyState {
    async fn get_serial(&self, _id: &'_ str) -> Option<SerialState> {
        panic!("Should not be called by OpenAPI generator.");
    }
    async fn get_serials(&self) -> Vec<SerialState> {
        panic!("Should not be called by OpenAPI generator.");
    }
    async fn get_serial_ids(&self) -> Vec<String> {
        panic!("Should not be called by OpenAPI generator.");
    }
}

#[tokio::main]
/// Generate the OpenAPI Spec for the `remote-usb` service.
async fn main() -> Result<(), u8> {
    let state = DummyState;
    let json = serde_json::to_value(api::get_api::<(), DummyState>(state).await.unwrap()).unwrap();
    let json_str = serde_json::to_string_pretty(&json).unwrap();
    println!("{}", json_str);

    Ok(())
}
