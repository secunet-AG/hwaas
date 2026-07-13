// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use hidg::Key;

/// Translate literal key names used by the user in the API to key code and
/// modifier that will be handed over to the hidg lib for further abstraction.
pub fn to_key(key: String) -> Result<(Key, u8), String> {
    let no_mod = 0x00;
    let left_shift_mod = 0x02;
    match key.as_str() {
        // No key
        "none" => Ok((Key::None, no_mod)),
        // Keyboard Error Roll Over - used for all slots if too many keys are pressed ("Phantom key".to_string())
        "overflow" => Ok((Key::Overflow, no_mod)),
        // Keyboard POST Fail
        "post-fail" => Ok((Key::PostFail, no_mod)),
        // Keyboard Error Undefined
        "undefined" => Ok((Key::Undefined, no_mod)),
        // Keyboard a and A
        "a" => Ok((Key::A, no_mod)),
        "A" => Ok((Key::A, left_shift_mod)),
        // Keyboard b and B
        "b" => Ok((Key::B, no_mod)),
        "B" => Ok((Key::B, left_shift_mod)),
        // Keyboard c and C
        "c" => Ok((Key::C, no_mod)),
        "C" => Ok((Key::C, left_shift_mod)),
        // Keyboard d and D
        "d" => Ok((Key::D, no_mod)),
        "D" => Ok((Key::D, left_shift_mod)),
        // Keyboard e and E
        "e" => Ok((Key::E, no_mod)),
        "E" => Ok((Key::E, left_shift_mod)),
        // Keyboard f and F
        "f" => Ok((Key::F, no_mod)),
        "F" => Ok((Key::F, left_shift_mod)),
        // Keyboard g and G
        "g" => Ok((Key::G, no_mod)),
        "G" => Ok((Key::G, left_shift_mod)),
        // Keyboard h and H
        "h" => Ok((Key::H, no_mod)),
        "H" => Ok((Key::H, left_shift_mod)),
        // Keyboard i and I
        "i" => Ok((Key::I, no_mod)),
        "I" => Ok((Key::I, left_shift_mod)),
        // Keyboard j and J
        "j" => Ok((Key::J, no_mod)),
        "J" => Ok((Key::J, left_shift_mod)),
        // Keyboard k and K
        "k" => Ok((Key::K, no_mod)),
        "K" => Ok((Key::K, left_shift_mod)),
        // Keyboard l and L
        "l" => Ok((Key::L, no_mod)),
        "L" => Ok((Key::L, left_shift_mod)),
        // Keyboard m and M
        "m" => Ok((Key::M, no_mod)),
        "M" => Ok((Key::M, left_shift_mod)),
        // Keyboard n and N
        "n" => Ok((Key::N, no_mod)),
        "N" => Ok((Key::N, left_shift_mod)),
        // Keyboard o and O
        "o" => Ok((Key::O, no_mod)),
        "O" => Ok((Key::O, left_shift_mod)),
        // Keyboard p and P
        "p" => Ok((Key::P, no_mod)),
        "P" => Ok((Key::P, left_shift_mod)),
        // Keyboard q and Q
        "q" => Ok((Key::Q, no_mod)),
        "Q" => Ok((Key::Q, left_shift_mod)),
        // Keyboard r and R
        "r" => Ok((Key::R, no_mod)),
        "R" => Ok((Key::R, left_shift_mod)),
        // Keyboard s and S
        "s" => Ok((Key::S, no_mod)),
        "S" => Ok((Key::S, left_shift_mod)),
        // Keyboard t and T
        "t" => Ok((Key::T, no_mod)),
        "T" => Ok((Key::T, left_shift_mod)),
        // Keyboard u and U
        "u" => Ok((Key::U, no_mod)),
        "U" => Ok((Key::U, left_shift_mod)),
        // Keyboard v and V
        "v" => Ok((Key::V, no_mod)),
        "V" => Ok((Key::V, left_shift_mod)),
        // Keyboard w and W
        "w" => Ok((Key::W, no_mod)),
        "W" => Ok((Key::W, left_shift_mod)),
        // Keyboard x and X
        "x" => Ok((Key::X, no_mod)),
        "X" => Ok((Key::X, left_shift_mod)),
        // Keyboard y and Y
        "y" => Ok((Key::Y, no_mod)),
        "Y" => Ok((Key::Y, left_shift_mod)),
        // Keyboard z and Z
        "z" => Ok((Key::Z, no_mod)),
        "Z" => Ok((Key::Z, left_shift_mod)),
        // Keyboard 1 and !
        "1" => Ok((Key::Num1, no_mod)),
        "!" => Ok((Key::Num1, left_shift_mod)),
        // Keyboard 2 and @
        "2" => Ok((Key::Num2, no_mod)),
        "@" => Ok((Key::Num2, left_shift_mod)),
        // Keyboard 3 and #
        "3" => Ok((Key::Num3, no_mod)),
        "#" => Ok((Key::Num3, left_shift_mod)),
        // Keyboard 4 and $
        "4" => Ok((Key::Num4, no_mod)),
        "$" => Ok((Key::Num4, left_shift_mod)),
        // Keyboard 5 and %
        "5" => Ok((Key::Num5, no_mod)),
        "%" => Ok((Key::Num5, left_shift_mod)),
        // Keyboard 6 and ^
        "6" => Ok((Key::Num6, no_mod)),
        "^" => Ok((Key::Num6, left_shift_mod)),
        // Keyboard 7 and &
        "7" => Ok((Key::Num7, no_mod)),
        "&" => Ok((Key::Num7, left_shift_mod)),
        // Keyboard 8 and *
        "8" => Ok((Key::Num8, no_mod)),
        "*" => Ok((Key::Num8, left_shift_mod)),
        // Keyboard 9 and (
        "9" => Ok((Key::Num9, no_mod)),
        "(" => Ok((Key::Num9, left_shift_mod)),
        // Keyboard 0 and )
        "0" => Ok((Key::Num0, no_mod)),
        ")" => Ok((Key::Num0, left_shift_mod)),
        // Keyboard Return (ENTER)
        "\n" => Ok((Key::Enter, no_mod)),
        // Keyboard ESCAPE
        "esc" => Ok((Key::Esc, no_mod)),
        // Keyboard DELETE (Backspace)
        "delete" => Ok((Key::BackSpace, no_mod)),
        // Keyboard Tab
        "tab" => Ok((Key::Tab, no_mod)),
        // Keyboard Spacebar
        " " => Ok((Key::Space, no_mod)),
        // Keyboard - and _
        "-" => Ok((Key::Minus, no_mod)),
        "_" => Ok((Key::Minus, left_shift_mod)),
        // Keyboard = and +
        "=" => Ok((Key::Equal, no_mod)),
        "+" => Ok((Key::Equal, left_shift_mod)),
        // Keyboard [ and {
        "[" => Ok((Key::LeftBrace, no_mod)),
        "{" => Ok((Key::LeftBrace, left_shift_mod)),
        // Keyboard ] and }
        "]" => Ok((Key::RightBrace, no_mod)),
        "}" => Ok((Key::RightBrace, left_shift_mod)),
        // Keyboard \ and |
        "\\" => Ok((Key::BackSlash, no_mod)),
        "|" => Ok((Key::BackSlash, left_shift_mod)),
        // 0x32 Keyboard Non-US # and ~
        // Keyboard ; and :
        ";" => Ok((Key::Semicolon, no_mod)),
        ":" => Ok((Key::Semicolon, left_shift_mod)),
        // Keyboard ' and "
        "'" => Ok((Key::Apostrophe, no_mod)),
        "\"" => Ok((Key::Apostrophe, left_shift_mod)),
        // Keyboard ` and ~
        "`" => Ok((Key::Grave, no_mod)),
        "~" => Ok((Key::Grave, left_shift_mod)),
        // Keyboard , and <
        "," => Ok((Key::Comma, no_mod)),
        "<" => Ok((Key::Comma, left_shift_mod)),
        // Keyboard . and >
        "." => Ok((Key::Dot, no_mod)),
        ">" => Ok((Key::Dot, left_shift_mod)),
        // Keyboard / and ?
        "/" => Ok((Key::Slash, no_mod)),
        "?" => Ok((Key::Slash, left_shift_mod)),
        // Keyboard Caps Lock
        "caps-lock" => Ok((Key::CapsLock, no_mod)),
        // Keyboard F1
        "f1" => Ok((Key::F1, no_mod)),
        // Keyboard F2
        "f2" => Ok((Key::F2, no_mod)),
        // Keyboard F3
        "f3" => Ok((Key::F3, no_mod)),
        // Keyboard F4
        "f4" => Ok((Key::F4, no_mod)),
        // Keyboard F5
        "f5" => Ok((Key::F5, no_mod)),
        // Keyboard F6
        "f6" => Ok((Key::F6, no_mod)),
        // Keyboard F7
        "f7" => Ok((Key::F7, no_mod)),
        // Keyboard F8
        "f8" => Ok((Key::F8, no_mod)),
        // Keyboard F9
        "f9" => Ok((Key::F9, no_mod)),
        // Keyboard F10
        "f10" => Ok((Key::F10, no_mod)),
        // Keyboard F11
        "f11" => Ok((Key::F11, no_mod)),
        // Keyboard F12
        "f12" => Ok((Key::F12, no_mod)),
        // Keyboard Print Screen
        "sysrq" => Ok((Key::SysRq, no_mod)),
        // Keyboard Scroll Lock
        "scroll-lock" => Ok((Key::ScrollLock, no_mod)),
        // Keyboard Pause
        "pause" => Ok((Key::Pause, no_mod)),
        // Keyboard Insert
        "insert" => Ok((Key::Insert, no_mod)),
        // Keyboard Home
        "home" => Ok((Key::Home, no_mod)),
        // Keyboard Page Up
        "page-up" => Ok((Key::PageUp, no_mod)),
        // Keyboard Delete Forward
        "delete-forward" => Ok((Key::Delete, no_mod)),
        // Keyboard End
        "end" => Ok((Key::End, no_mod)),
        // Keyboard Page Down
        "page-down" => Ok((Key::PageDown, no_mod)),
        // Keyboard Right Arrow
        "right" => Ok((Key::Right, no_mod)),
        // Keyboard Left Arrow
        "left" => Ok((Key::Left, no_mod)),
        // Keyboard Down Arrow
        "down" => Ok((Key::Down, no_mod)),
        // Keyboard Up Arrow
        "up" => Ok((Key::Up, no_mod)),
        // Keyboard Num Lock and Clear
        "num-lock" => Ok((Key::NumLock, no_mod)),
        // Keypad /
        "keypad-slash" => Ok((Key::KeyPadSlash, no_mod)),
        // Keypad *
        "keypad-asterisk" => Ok((Key::KeyPadAsterisk, no_mod)),
        // Keypad -
        "keypad-minus" => Ok((Key::KeyPadMinus, no_mod)),
        // Keypad +
        "keypad-plus" => Ok((Key::KeyPadPlus, no_mod)),
        // Keypad ENTER
        "keypad-enter" => Ok((Key::KeyPadEnter, no_mod)),
        // Keypad 1 and End
        "keypad-1" => Ok((Key::KeyPad1, no_mod)),
        // Keypad 2 and Down Arrow
        "keypad-2" => Ok((Key::KeyPad2, no_mod)),
        // Keypad 3 and PageDn
        "keypad-3" => Ok((Key::KeyPad3, no_mod)),
        // Keypad 4 and Left Arrow
        "keypad-4" => Ok((Key::KeyPad4, no_mod)),
        // Keypad 5
        "keypad-5" => Ok((Key::KeyPad5, no_mod)),
        // Keypad 6 and Right Arrow
        "keypad-6" => Ok((Key::KeyPad6, no_mod)),
        // Keypad 7 and Home
        "keypad-7" => Ok((Key::KeyPad7, no_mod)),
        // Keypad 8 and Up Arrow
        "keypad-8" => Ok((Key::KeyPad8, no_mod)),
        // Keypad 9 and Page Up
        "keypad-9" => Ok((Key::KeyPad9, no_mod)),
        // Keypad 0 and Insert
        "keypad-0" => Ok((Key::KeyPad0, no_mod)),
        // Keypad . and Delete
        "keypad-dot" => Ok((Key::KeyPadDot, no_mod)),
        // Keyboard Non-US \ and |
        "nonus-backslash" => Ok((Key::NonUsBackSlash, no_mod)),
        // Keyboard Application
        "compose" => Ok((Key::Compose, no_mod)),
        // Keyboard Power
        "power" => Ok((Key::Power, no_mod)),
        // Keypad =
        "keypad-equal" => Ok((Key::KeyPadEqual, no_mod)),
        // Keyboard F13
        "f13" => Ok((Key::F13, no_mod)),
        // Keyboard F14
        "f14" => Ok((Key::F14, no_mod)),
        // Keyboard F15
        "f15" => Ok((Key::F15, no_mod)),
        // Keyboard F16
        "f16" => Ok((Key::F16, no_mod)),
        // Keyboard F17
        "f17" => Ok((Key::F17, no_mod)),
        // Keyboard F18
        "f18" => Ok((Key::F18, no_mod)),
        // Keyboard F19
        "f19" => Ok((Key::F19, no_mod)),
        // Keyboard F20
        "f20" => Ok((Key::F20, no_mod)),
        // Keyboard F21
        "f21" => Ok((Key::F21, no_mod)),
        // Keyboard F22
        "f22" => Ok((Key::F22, no_mod)),
        // Keyboard F23
        "f23" => Ok((Key::F23, no_mod)),
        // Keyboard F24
        "f24" => Ok((Key::F24, no_mod)),
        // Keyboard Execute
        "open" => Ok((Key::Open, no_mod)),
        // Keyboard Help
        "help" => Ok((Key::Help, no_mod)),
        // Keyboard Menu
        "props" => Ok((Key::Props, no_mod)),
        // Keyboard Select
        "front" => Ok((Key::Front, no_mod)),
        // Keyboard Stop
        "stop" => Ok((Key::Stop, no_mod)),
        // Keyboard Again
        "again" => Ok((Key::Again, no_mod)),
        // Keyboard Undo
        "undo" => Ok((Key::Undo, no_mod)),
        // Keyboard Cut
        "cut" => Ok((Key::Cut, no_mod)),
        // Keyboard Copy
        "copy" => Ok((Key::Copy, no_mod)),
        // Keyboard Paste
        "paste" => Ok((Key::Paste, no_mod)),
        // Keyboard Find
        "find" => Ok((Key::Find, no_mod)),
        // Keyboard Mute
        "mute" => Ok((Key::Mute, no_mod)),
        // Keyboard Volume Up
        "volume-up" => Ok((Key::VolumeUp, no_mod)),
        // Keyboard Volume Down
        "volume-down" => Ok((Key::VolumeDown, no_mod)),
        // Keyboard Locking Caps Lock
        "locking-caps-lock" => Ok((Key::LockingCapsLock, no_mod)),
        // Keyboard Locking Num Lock
        "locking-num-lock" => Ok((Key::LockingNumLock, no_mod)),
        // Keyboard Locking Scroll Lock
        "locking-scroll-lock" => Ok((Key::LockingScrollLock, no_mod)),
        // Keypad Comma
        "keypad-comma" => Ok((Key::KeyPadComma, no_mod)),
        // Keypad Equal Sign
        "keypad-equal-sign" => Ok((Key::KeyPadEqualSign, no_mod)),
        // Keyboard International1
        "ro" => Ok((Key::Ro, no_mod)),
        // Keyboard International2
        "katakana-hiragana" => Ok((Key::KatakanaHiragana, no_mod)),
        // Keyboard International3
        "yen" => Ok((Key::Yen, no_mod)),
        // Keyboard International4
        "henkan" => Ok((Key::Henkan, no_mod)),
        // Keyboard International5
        "munenkan" => Ok((Key::Munenkan, no_mod)),
        // Keyboard International6
        "keypad-jp-comma" => Ok((Key::KeyPadJpComma, no_mod)),
        // Keyboard LANG1
        "hangeul" => Ok((Key::Hangeul, no_mod)),
        // Keyboard LANG2
        "hanja" => Ok((Key::Hanja, no_mod)),
        // Keyboard LANG3
        "katakana" => Ok((Key::Katakana, no_mod)),
        // Keyboard LANG4
        "hiragana" => Ok((Key::Hiragana, no_mod)),
        // Keyboard LANG5
        "zenkaku-hankaku" => Ok((Key::ZankakuHankaku, no_mod)),
        // Keypad (
        "keypad-left-paren" => Ok((Key::KeyPadLeftParen, no_mod)),
        // Keypad )
        "keypad-right-paren" => Ok((Key::KeyPadRightParen, no_mod)),

        // Keyboard Left Control
        "left-ctrl" => Ok((Key::LeftCtrl, 0x01)),
        // Keyboard Left Shift
        "left-shift" => Ok((Key::LeftShift, 0x02)),
        // Keyboard Left Alt
        "left-alt" => Ok((Key::LeftAlt, 0x04)),
        // Keyboard Left GUI
        "left-meta" => Ok((Key::LeftMeta, 0x08)),
        // Keyboard Right Control
        "right-ctrl" => Ok((Key::RightCtrl, 0x10)),
        // Keyboard Right Shift
        "right-shift" => Ok((Key::RightShift, 0x20)),
        // Keyboard Right Alt
        "right-alt" => Ok((Key::RightAlt, 0x40)),
        // Keyboard Right GUI
        "right-meta" => Ok((Key::RightMeta, 0x80)),

        // Unknown user input
        _ => Err(key),
    }
}
