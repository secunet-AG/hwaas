// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

pub mod buttons;
pub mod key;

/// Keyboard report descriptor used to configure a virtual USB OTG keyboard.
pub const DEFAULT_KEYBOARD_REPORT_DESCRIPTOR: [u8; 63] = [
    0x05, 0x01, // Usage Page (Generic Desktop Ctrls)
    0x09, 0x06, // Usage (Keyboard)
    0xa1, 0x01, // Collection (Application)
    0x05, 0x07, // Usage Page (Kbrd/Keypad)
    0x19, 0xe0, // Usage Minimum (0xE0)
    0x29, 0xe7, // Usage Maximum (0xE7)
    0x15, 0x00, // Logical Minimum (0)
    0x25, 0x01, // Logical Maximum (1)
    0x75, 0x01, // Report Size (1)
    0x95, 0x08, // Report Count (8)
    0x81, 0x02, // Input (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position)
    0x95, 0x01, // Report Count (1)
    0x75, 0x08, // Report Size (8)
    0x81, 0x03, // Input (Const,Var,Abs,No Wrap,Linear,Preferred State,No Null Position)
    0x95, 0x05, // Report Count (5)
    0x75, 0x01, // Report Size (1)
    0x05, 0x08, // Usage Page (LEDs)
    0x19, 0x01, // Usage Minimum (Num Lock)
    0x29, 0x05, // Usage Maximum (Kana)
    0x91, 0x02, // Output (Data,Var,Abs)
    0x95, 0x01, // Report Count (1)
    0x75, 0x03, // Report Size (3)
    0x91, 0x03, // Output (Const,Var,Abs)
    0x95, 0x06, // Report Count (6)
    0x75, 0x08, // Report Size (8)
    0x15, 0x00, // Logical Minimum (0)
    0x25, 0x65, // Logical Maximum (101)
    0x05, 0x07, // Usage Page (Kbrd/Keypad)
    0x19, 0x00, // Usage Minimum (0x00)
    0x29, 0x65, // Usage Maximum (0x65)
    0x81, 0x00, // Input (Data,Array,Abs,No Wrap,Linear,Preferred State,No Null Position)
    0xc0, // End Collection
];
/// Keyboard report length used to configure a virtual USB OTG keyboard.
pub const DEFAULT_KEYBOARD_REPORT_LENGTH: u8 = 8;

/// Mouse report descriptor used to configure a virtual USB OTG mouse.
pub const DEFAULT_MOUSE_REPORT_DESCRIPTOR: [u8; 66] = [
    0x05, 0x01, // USAGE_PAGE (Generic Desktop)
    0x09, 0x02, // USAGE (Mouse)
    0xa1, 0x01, // COLLECTION (Application)
    0x09, 0x01, //   USAGE (Pointer)
    0xa1, 0x00, //   COLLECTION (Physical)
    0x05, 0x09, //     USAGE_PAGE (Button)
    0x19, 0x01, //     USAGE_MINIMUM (Button 1)
    0x29, 0x03, //     USAGE_MAXIMUM (Button 3)
    0x15, 0x00, //     LOGICAL_MINIMUM (0)
    0x25, 0x01, //     LOGICAL_MAXIMUM (1)
    0x95, 0x03, //     REPORT_COUNT (3)
    0x75, 0x01, //     REPORT_SIZE (1)
    0x81, 0x02, //     INPUT (Data,Var,Abs)
    0x95, 0x01, //     REPORT_COUNT (1)
    0x75, 0x05, //     REPORT_SIZE (5)
    0x81, 0x03, //     INPUT (Cnst,Var,Abs)
    0x05, 0x01, //     USAGE_PAGE (Generic Desktop)
    0x09, 0x30, //     USAGE (X)
    0x09, 0x31, //     USAGE (Y)
    0x16, 0x01, 0x80, // Logical Minimum (-32767)
    0x26, 0xFF, 0x7F, // Logical Maximum (32767)
    0x95, 0x02, //     REPORT_COUNT (2)
    0x75, 0x10, //     REPORT_SIZE (16)
    0x81, 0x06, //     INPUT (Data,Var,Rel)
    0x05, 0x01, //     USAGE_PAGE (Generic Desktop)
    0x09, 0x38, //     USAGE (Wheel)
    0x15, 0x81, //     LOGICAL_MINIMUM (-127)
    0x25, 0x7f, //     LOGICAL_MAXIMUM (127)
    0x95, 0x01, //     REPORT_COUNT (1)
    0x75, 0x08, //     REPORT_SIZE (8)
    0x81, 0x06, //     INPUT (Data,Var,Rel)
    0xc0, //         END_COLLECTION
    0xc0, //       END_COLLECTION
];
/// Mouse report length used to configure a virtual USB OTG mouse.
pub const DEFAULT_MOUSE_REPORT_LENGTH: u8 = 16;
