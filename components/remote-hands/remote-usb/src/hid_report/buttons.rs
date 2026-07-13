// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use hidg::Button;

/// Translate button input used by the user in the API to the `Button` type
/// of the `hidg` lib.
pub fn to_buttons(input: Vec<u8>) -> Result<Vec<Button>, &'static str> {
    let mut button_list = vec![];
    for b in input {
        button_list.push(Button::safe_from(b).ok_or(
            "cannot parse '{b}' to Button, only support: 0 - right, 1 - left, 2 - middle",
        )?);
    }
    Ok(button_list)
}
