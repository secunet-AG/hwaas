// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use aide::{
    axum::{
        ApiRouter,
        routing::{get_with, post_with},
    },
    transform::{TransformOperation, TransformPathItem},
};
use axum::{
    Json,
    extract::{
        State, WebSocketUpgrade,
        ws::{CloseFrame, Message, WebSocket},
    },
    http::StatusCode,
    response::Response,
};
use hidg::{
    Button,
    Key::{self},
    Modifiers,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::error;

use crate::{
    app_state::UsbConfigurable,
    hid_report::{buttons::to_buttons, key::to_key},
};

use super::json_error;

// Only exists for serde default usage.
// Waiting for serde to enable `default_value` or alike.
const fn serde_default_true() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
/// User input for keyboard as String.
/// If newline is set to True, '\n' is appended to the String.
pub struct KeyboardText {
    input: String,
    #[serde(default = "serde_default_true")]
    newline: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
/// User input for keyboard as a list of Keys.
/// A maximum of 6 keys can be pressed at the same time.
/// The modifier is used for keys like "shift" and does not count to the limit.
/// All keys can be pressed and/or released, but one must be given.
/// A short description of report format can be found here:
/// https://wiki.osdev.org/USB_Human_Interface_Devices#usb_keyboard.
pub struct KeyboardReport {
    pub keys: Vec<String>,
    pub modifier: u8,
    #[serde(default = "serde_default_true")]
    pub press: bool,
    #[serde(default = "serde_default_true")]
    pub release: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
/// User input for mouse.
pub struct MouseReport {
    /// Buttons to be pressed. Supported are a maximum of 3: 0 - right, 1 - left, 2 - middle.
    pub buttons: Vec<u8>,
    /// Absolute coordinates to move the pointer to
    #[serde(rename = "x")]
    pub pointer_x: u16,
    #[serde(rename = "y")]
    pub pointer_y: u16,
    /// Coordinate to set the mouse wheel to
    pub wheel: i8,
}

/// Router for requests to the usb hid sub routes
pub fn hid_router<T: UsbConfigurable>() -> ApiRouter<T> {
    ApiRouter::new()
        .api_route_with(
            "/usb/keyboard/text",
            post_with(
                handle_post_keyboard_text::<T>,
                handle_post_keyboard_text_doc,
            ),
            usb_api_doc,
        )
        .api_route_with(
            "/usb/keyboard/report",
            post_with(
                handle_post_keyboard_report::<T>,
                handle_post_keyboard_report_doc,
            ),
            usb_api_doc,
        )
        .api_route_with(
            "/usb/keyboard/websocket",
            get_with(
                handle_keyboard_report_socket::<T>,
                handle_keyboard_report_socket_doc,
            ),
            usb_api_doc,
        )
        .api_route_with(
            "/usb/mouse",
            post_with(handle_post_mouse::<T>, handle_post_mouse_doc),
            usb_api_doc,
        )
        .api_route_with(
            "/usb/mouse/websocket",
            get_with(handle_mouse_websocket::<T>, handle_mouse_websocket_doc),
            usb_api_doc,
        )
}

/// The tag for the UsbAPI. Used for (un-)folding this API in the UI.
fn usb_api_doc(op: TransformPathItem) -> TransformPathItem {
    op.tag("USB API")
}

/// POST handler to send text to the USB OTG keyboard. Does one key at a time.
async fn handle_post_keyboard_text<T: UsbConfigurable>(
    State(state): State<T>,
    Json(KeyboardText { input, newline }): Json<KeyboardText>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    // send each character of the given text to the keyboard
    for c in input.chars() {
        // convert input to USB OTG readable data structure first
        let (keycode, modifier) = to_key(c.to_string()).map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                json_error(format!("unknown input '{:?}'", e)),
            )
        })?;
        // generate mask from given modifier
        let modifier_mask = Modifiers::safe_from(modifier);
        // write to keyboard with "press: true" and "release: true" since
        // this API is for simple text handling
        state
            .use_keyboard(modifier_mask, vec![keycode], true, true)
            .await
            .map_err(|e| {
                error!(
                    char = c.to_string(),
                    error.msg = %e,
                    error.dbg = ?e,
                    "an error occurred when writing to keyboard"
                );
                (StatusCode::INTERNAL_SERVER_ERROR, json_error(e))
            })?;
    }
    // optionally send a newline after the input
    if newline {
        let (keycode, modifier) = to_key('\n'.to_string()).map_err(|_| {
            error!("bug: was not able to transform '\\n'!");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json_error("bug: this shoudn't happen!"),
            )
        })?;

        let modifier_mask = Modifiers::safe_from(modifier);
        state
            .use_keyboard(modifier_mask, vec![keycode], true, true)
            .await
            .map_err(|e| {
                error!(
                    char = "\n",
                    error.msg = %e,
                    error.dbg = ?e,
                    "an error occurred when writing to keyboard"
                );
                (StatusCode::INTERNAL_SERVER_ERROR, json_error(e))
            })?;
    }
    Ok(StatusCode::OK)
}

/// Documentation for the POST text handler.
/// Expected errors are `StatusCode::BAD_REQUEST` (on unknown input) and
/// `StatusCode::INTERNAL_SERVER_ERROR` on issues when writing to the keyboard.
fn handle_post_keyboard_text_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Use keyboard with plain text")
        .description(
            "Use USB OTG keyboard by sending plain text input. \
            Each character is send individually. An optional newline can be send afterwards.",
        )
        .response::<200, ()>()
        .response_with::<400, Json<Value>, _>(|op| op.description("Unknown input"))
        .response_with::<500, Json<Value>, _>(|op| op.description("Error writing to the keyboard"))
}

/// POST handler to send a report to the USB OTG keyboard.
async fn handle_post_keyboard_report<T: UsbConfigurable>(
    State(state): State<T>,
    Json(KeyboardReport {
        keys,
        modifier,
        press,
        release,
    }): Json<KeyboardReport>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    // check maximum of keys that can be pressed in parallel
    let key_amount = keys.len();
    let is_less_than_maximum_keys = key_amount <= 6;
    // check that the keys are getting at least pressed or released
    let at_least_one_action = [press, release].iter().any(|&b| b);

    if is_less_than_maximum_keys && at_least_one_action {
        // convert input to USB OTG readable data structure
        let mut hidg_keys: Vec<Key> = vec![];
        let mut modifiers: Vec<u8> = vec![];
        for key in &keys {
            let (keycode, modifier) = to_key(key.clone()).map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    json_error(format!("unknown input '{:?}'", e)),
                )
            })?;
            hidg_keys.push(keycode);
            modifiers.push(modifier);
        }
        // calculate modifier depending on the API input:
        // combine given modifier with modifiers retrieved from given keys
        // Example: modifier can be set to 0 but capital A is given, which
        // sets the modifier to 0x02 (left-shift)
        let mut hidg_modifier = modifier;
        for modi in modifiers {
            hidg_modifier |= modi;
        }
        let modifier_mask = Modifiers::safe_from(hidg_modifier);
        // use keyboard
        state
            .use_keyboard(modifier_mask, hidg_keys, press, release)
            .await
            .map_err(|e| {
                error!(
                    keys = format!("{keys:?}"),
                    error.msg = %e,
                    error.dbg = ?e,
                    "an error occurred when writing to keyboard"
                );
                (StatusCode::INTERNAL_SERVER_ERROR, json_error(e))
            })?;
    } else {
        let mut err_msg = "";
        if !is_less_than_maximum_keys {
            err_msg = "found more than 6 keys, but USB only allows for 6 keys to be pressed at the same time.";
            error!(
                key_amount,
                error.msg = %err_msg,
                error.dbg = ?err_msg,
                "an error occurred when writing to keyboard"
            );
        } else if !at_least_one_action {
            err_msg = "neither press nor release are enabled!";
            error!(
                error.msg = %err_msg,
                error.dbg = ?err_msg,
                "an error occurred when writing to keyboard"
            );
        }
        return Err((StatusCode::BAD_REQUEST, json_error(err_msg)));
    }
    Ok(StatusCode::OK)
}

/// Documentation for the POST report handler.
/// Expected errors are `StatusCode::BAD_REQUEST` (on unknown input) and
/// `StatusCode::INTERNAL_SERVER_ERROR` on issues when writing to the keyboard.
fn handle_post_keyboard_report_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Use keyboard with report format")
        .description(
            "Use USB OTG keyboard by sending report input - \
            list of keys, modifier, press and release values.",
        )
        .response::<200, ()>()
        .response_with::<400, Json<Value>, _>(|op| op.description("Unknown input"))
        .response_with::<500, Json<Value>, _>(|op| op.description("Error writing to the keyboard"))
}

/// Websocket handler to send a report to the USB OTG keyboard.
async fn handle_keyboard_report_socket<T: UsbConfigurable>(
    State(state): State<T>,
    ws_upgrade: WebSocketUpgrade,
) -> Response {
    ws_upgrade.on_upgrade(|socket| keyboard_report_socket(state, socket))
}

async fn keyboard_report_socket<T: UsbConfigurable>(state: T, mut ws: WebSocket) {
    while let Some(msg) = ws.recv().await {
        let msg = match msg {
            Ok(msg) => msg,
            Err(_) => return,
        };

        let bytes = msg.into_data();

        if bytes.is_empty() {
            continue;
        }

        let report: KeyboardReport = match serde_json::from_slice(&bytes) {
            Ok(r) => r,
            Err(e) => {
                let _ = ws.send(error_msg(format!("invalid report: {e}"))).await;
                continue;
            }
        };

        let num_keys = report.keys.len();

        let is_less_than_maximum_keys = num_keys <= 6;
        let at_least_one_action = [report.press, report.release].iter().any(|&b| b);

        // "Happy path", we can send a messaage to the underling device
        if is_less_than_maximum_keys && at_least_one_action {
            // convert input to USB OTG readable data structure
            let mut hidg_keys: Vec<Key> = vec![];
            let mut modifiers: Vec<u8> = vec![];
            for key in &report.keys {
                let (keycode, modifier) = match to_key(key.clone()) {
                    Ok(inner) => inner,
                    Err(e) => {
                        let _ = ws.send(error_msg(format!("unknown input: {e:?}"))).await;
                        continue;
                    }
                };

                hidg_keys.push(keycode);
                modifiers.push(modifier);
            }
            // calculate modifier depending on the API input:
            // combine given modifier with modifiers retrieved from given keys
            // Example: modifier can be set to 0 but capital A is given, which
            // sets the modifier to 0x02 (left-shift)
            let mut hidg_modifier = report.modifier;
            for modi in modifiers {
                hidg_modifier |= modi;
            }
            let modifier_mask = Modifiers::safe_from(hidg_modifier);
            // use keyboard
            if let Err(e) = state
                .use_keyboard(modifier_mask, hidg_keys, report.press, report.release)
                .await
            {
                error!(
                        keys = format!("{:?}", report.keys),
                        error.msg = %e,
                        error.dbg = ?e,
                        "an error occurred when writing to keyboard"
                );
                let _ = ws
                    .send(error_msg(
                        "an error occurred when writing to keyboard".to_string(),
                    ))
                    .await;
            }
        } else {
            let mut err_msg = "";
            if !is_less_than_maximum_keys {
                err_msg = "found more than 6 keys, but USB only allows for 6 keys to be pressed at the same time.";
                error!(
                    num_keys,
                    error.msg = %err_msg,
                    error.dbg = ?err_msg,
                    "an error occurred when writing to keyboard"
                );
            } else if !at_least_one_action {
                err_msg = "neither press nor release are enabled!";
                error!(
                    error.msg = %err_msg,
                    error.dbg = ?err_msg,
                    "an error occurred when writing to keyboard"
                );
            }
            let _ = ws.send(error_msg(err_msg)).await;
        }
    }
}

/// Documentation for the WS report handler.
fn handle_keyboard_report_socket_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Use keyboard with report format via WS")
        .description(
            "Use USB OTG keyboard by sending report input - \
            list of keys, modifier, press and release values.",
        )
}

/// POST handler to send a report to the USB OTG mouse.
async fn handle_post_mouse<T: UsbConfigurable>(
    State(state): State<T>,
    Json(MouseReport {
        buttons,
        pointer_x,
        pointer_y,
        wheel,
    }): Json<MouseReport>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    // check maximum of mouse buttons
    let button_amount = buttons.len();
    let is_less_than_maximum_buttons = button_amount <= 3;

    if is_less_than_maximum_buttons {
        // convert input to USB OTG readable data structure
        let hidg_buttons: Vec<Button> = to_buttons(buttons).map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                json_error(format!("unknown input: '{:?}'", e)),
            )
        })?;
        let pointer = (pointer_x, pointer_y);
        state
            .use_mouse(hidg_buttons.clone(), pointer, wheel)
            .await
            .map_err(|e| {
                error!(
                    buttons = format!("{hidg_buttons:?}"),
                    pointer = format!("{pointer:?}"),
                    wheel = format!("{wheel:?}"),
                    error.msg = %e,
                    error.dbg = ?e,
                    "an error occurred when writing to mouse"
                );
                (StatusCode::INTERNAL_SERVER_ERROR, json_error(e))
            })?;
    } else {
        let err_msg =
            "found more than 3 buttons, but only right, left and middle button are supported.";
        error!(
            button_amount,
            error.msg = %err_msg,
            error.dbg = ?err_msg,
            "an error occurred when writing to mouse"
        );
        return Err((StatusCode::BAD_REQUEST, json_error(err_msg)));
    }
    Ok(StatusCode::OK)
}

/// Documentation for the POST mouse handler.
///
/// Expected errors are `StatusCode::BAD_REQUEST` (on unknown input) and
/// `StatusCode::INTERNAL_SERVER_ERROR` on issues when writing to the mouse.
fn handle_post_mouse_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Use mouse")
        .description("Use USB OTG mouse by sending button, pointer (x, y) and wheel parameters.")
        .response::<200, ()>()
        .response_with::<400, Json<Value>, _>(|op| op.description("Unknown input"))
        .response_with::<500, Json<Value>, _>(|op| op.description("Error writing to the mouse"))
}

/// WebSocket handler to send a report to the USB OTG mouse.
///
/// This closely follows the REST implementation.
async fn handle_mouse_websocket<T: UsbConfigurable>(
    State(state): State<T>,
    ws_upgrade: WebSocketUpgrade,
) -> Response {
    ws_upgrade.on_upgrade(|socket| mouse_socket(state, socket))
}

/// Documentation for the WS mouse handler.i
///
/// Error messages are sent back via the WS connection, and traced internally.
fn handle_mouse_websocket_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Use mouse via WS/WSS connection")
        .description("Use USB OTG mouse by sending button, pointer (x, y) and wheel parameters.")
}

/// The function responsible for handling and firing off WS mouse events
async fn mouse_socket<T: UsbConfigurable>(state: T, mut ws: WebSocket) {
    while let Some(msg) = ws.recv().await {
        let msg = match msg {
            Ok(msg) => msg,
            Err(_) => return,
        };

        let bytes = msg.into_data();

        if bytes.is_empty() {
            continue;
        }

        // Could not parse frame
        let report: MouseReport = match serde_json::from_slice(&bytes) {
            Ok(r) => r,
            Err(e) => {
                let _ = ws.send(error_msg(format!("invalid report: {e}"))).await;
                continue;
            }
        };

        // Max three buttons supported
        if report.buttons.len() > 3 {
            let _ = ws.send(error_msg("more than 3 buttons unsupported")).await;
            continue;
        }

        // Unknown input
        let hidg_buttons = match to_buttons(report.buttons) {
            Ok(b) => b,
            Err(e) => {
                let _ = ws.send(error_msg(format!("unknown input: {e:?}"))).await;
                continue;
            }
        };

        // Fire off the underlying mouse event, log an error otherwise
        if let Err(e) = state
            .use_mouse(
                hidg_buttons,
                (report.pointer_x, report.pointer_y),
                report.wheel,
            )
            .await
        {
            error!(error.msg = %e, error.dbg = ?e, "mouse write failed");
            let _ = ws
                .send(Message::Close(Some(CloseFrame {
                    code: 1011, // internal error
                    reason: "mouse write failed".into(),
                })))
                .await;
        };
    }
}

fn error_msg(text: impl Into<String>) -> Message {
    Message::Text(serde_json::json!({ "error": text.into() }).to_string())
}
