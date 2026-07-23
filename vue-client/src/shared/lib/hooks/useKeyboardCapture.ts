import { onBeforeUnmount, ref, type Ref } from 'vue'

export interface KeyboardReport {
  keys: string[]
  modifier: number
  press: boolean
  release: boolean
}

type SendFn = (payload: KeyboardReport, policy: 'reliable') => void

export function useKeyboardCapture(active: Ref<boolean>, send: SendFn) {
  const overflow = ref(false)
  const held = new Set<string>()

  function emit(press: boolean, release: boolean) {
    const keys = nonModifierKeys(held)
    if (keys === null) {
      overflow.value = true
      return
    }
    overflow.value = false

    send({ keys, modifier: modifierByte(held), press, release }, 'reliable')
  }

  // Filter our modifier keys, if we have too many keys held, return null
  function nonModifierKeys(held: Iterable<string>): string[] | null {
    const keys: string[] = []
    for (const code of held) {
      if (isModifier(code)) continue
      const name = CODE_TO_KEY[code] // Cast to our Rust crate enums
      if (name !== undefined) keys.push(name)
    }
    return keys.length > 6 ? null : keys
  }

  function onKeyDown(e: KeyboardEvent) {
    if (!active.value) return
    e.preventDefault()
    if (e.repeat) return // remote handles its own key repeat
    if (held.has(e.code)) return
    held.add(e.code)
    emit(true, false)
  }

  function onKeyUp(e: KeyboardEvent) {
    if (!active.value) return
    e.preventDefault()
    if (!held.delete(e.code)) return

    emit(false, true)
  }

  // Used when we drop the window
  function releaseAll() {
    if (held.size === 0) return
    held.clear()
    send({ keys: [], modifier: 0, press: false, release: true }, 'reliable')
    overflow.value = false
  }

  window.addEventListener('keydown', onKeyDown, true)
  window.addEventListener('keyup', onKeyUp, true)
  window.addEventListener('blur', releaseAll)

  onBeforeUnmount(() => {
    window.removeEventListener('keydown', onKeyDown, true)
    window.removeEventListener('keyup', onKeyUp, true)
    window.removeEventListener('blur', releaseAll)
    releaseAll()
  })

  return { overflow, releaseAll, isModifier }
}

export const MODIFIER_BITS: Record<string, number> = {
  ControlLeft: 0x01,
  ShiftLeft: 0x02,
  AltLeft: 0x04,
  MetaLeft: 0x08,
  ControlRight: 0x10,
  ShiftRight: 0x20,
  AltRight: 0x40,
  MetaRight: 0x80,
}

export const HID_USAGE: Record<string, number> = {
  KeyA: 0x04,
  KeyB: 0x05,
  KeyC: 0x06,
  KeyD: 0x07,
  KeyE: 0x08,
  KeyF: 0x09,
  KeyG: 0x0a,
  KeyH: 0x0b,
  KeyI: 0x0c,
  KeyJ: 0x0d,
  KeyK: 0x0e,
  KeyL: 0x0f,
  KeyM: 0x10,
  KeyN: 0x11,
  KeyO: 0x12,
  KeyP: 0x13,
  KeyQ: 0x14,
  KeyR: 0x15,
  KeyS: 0x16,
  KeyT: 0x17,
  KeyU: 0x18,
  KeyV: 0x19,
  KeyW: 0x1a,
  KeyX: 0x1b,
  KeyY: 0x1c,
  KeyZ: 0x1d,
  Digit1: 0x1e,
  Digit2: 0x1f,
  Digit3: 0x20,
  Digit4: 0x21,
  Digit5: 0x22,
  Digit6: 0x23,
  Digit7: 0x24,
  Digit8: 0x25,
  Digit9: 0x26,
  Digit0: 0x27,
  Enter: 0x28,
  Escape: 0x29,
  Backspace: 0x2a,
  Tab: 0x2b,
  Space: 0x2c,
  Minus: 0x2d,
  Equal: 0x2e,
  BracketLeft: 0x2f,
  BracketRight: 0x30,
  Backslash: 0x31,
  Semicolon: 0x33,
  Quote: 0x34,
  Backquote: 0x35,
  Comma: 0x36,
  Period: 0x37,
  Slash: 0x38,
  CapsLock: 0x39,
  F1: 0x3a,
  F2: 0x3b,
  F3: 0x3c,
  F4: 0x3d,
  F5: 0x3e,
  F6: 0x3f,
  F7: 0x40,
  F8: 0x41,
  F9: 0x42,
  F10: 0x43,
  F11: 0x44,
  F12: 0x45,
  PrintScreen: 0x46,
  ScrollLock: 0x47,
  Pause: 0x48,
  Insert: 0x49,
  Home: 0x4a,
  PageUp: 0x4b,
  Delete: 0x4c,
  End: 0x4d,
  PageDown: 0x4e,
  ArrowRight: 0x4f,
  ArrowLeft: 0x50,
  ArrowDown: 0x51,
  ArrowUp: 0x52,
  NumLock: 0x53,
  NumpadDivide: 0x54,
  NumpadMultiply: 0x55,
  NumpadSubtract: 0x56,
  NumpadAdd: 0x57,
  NumpadEnter: 0x58,
  Numpad1: 0x59,
  Numpad2: 0x5a,
  Numpad3: 0x5b,
  Numpad4: 0x5c,
  Numpad5: 0x5d,
  Numpad6: 0x5e,
  Numpad7: 0x5f,
  Numpad8: 0x60,
  Numpad9: 0x61,
  Numpad0: 0x62,
  NumpadDecimal: 0x63,
  ContextMenu: 0x65,
}

// Maps browser KeyboardEvent.code to the strings to_key() accepts.
// Modifiers are intentionally absent — they travel in the modifier byte.
export const CODE_TO_KEY: Record<string, string> = {
  KeyA: 'a',
  KeyB: 'b',
  KeyC: 'c',
  KeyD: 'd',
  KeyE: 'e',
  KeyF: 'f',
  KeyG: 'g',
  KeyH: 'h',
  KeyI: 'i',
  KeyJ: 'j',
  KeyK: 'k',
  KeyL: 'l',
  KeyM: 'm',
  KeyN: 'n',
  KeyO: 'o',
  KeyP: 'p',
  KeyQ: 'q',
  KeyR: 'r',
  KeyS: 's',
  KeyT: 't',
  KeyU: 'u',
  KeyV: 'v',
  KeyW: 'w',
  KeyX: 'x',
  KeyY: 'y',
  KeyZ: 'z',
  Digit1: '1',
  Digit2: '2',
  Digit3: '3',
  Digit4: '4',
  Digit5: '5',
  Digit6: '6',
  Digit7: '7',
  Digit8: '8',
  Digit9: '9',
  Digit0: '0',
  Enter: '\n',
  Escape: 'esc',
  Backspace: 'delete',
  Tab: 'tab',
  Space: ' ',
  Minus: '-',
  Equal: '=',
  BracketLeft: '[',
  BracketRight: ']',
  Backslash: '\\',
  Semicolon: ';',
  Quote: "'",
  Backquote: '`',
  Comma: ',',
  Period: '.',
  Slash: '/',
  CapsLock: 'caps-lock',
  F1: 'f1',
  F2: 'f2',
  F3: 'f3',
  F4: 'f4',
  F5: 'f5',
  F6: 'f6',
  F7: 'f7',
  F8: 'f8',
  F9: 'f9',
  F10: 'f10',
  F11: 'f11',
  F12: 'f12',
  F13: 'f13',
  F14: 'f14',
  F15: 'f15',
  F16: 'f16',
  F17: 'f17',
  F18: 'f18',
  F19: 'f19',
  F20: 'f20',
  F21: 'f21',
  F22: 'f22',
  F23: 'f23',
  F24: 'f24',
  PrintScreen: 'sysrq',
  ScrollLock: 'scroll-lock',
  Pause: 'pause',
  Insert: 'insert',
  Home: 'home',
  PageUp: 'page-up',
  Delete: 'delete-forward',
  End: 'end',
  PageDown: 'page-down',
  ArrowRight: 'right',
  ArrowLeft: 'left',
  ArrowDown: 'down',
  ArrowUp: 'up',
  NumLock: 'num-lock',
  NumpadDivide: 'keypad-slash',
  NumpadMultiply: 'keypad-asterisk',
  NumpadSubtract: 'keypad-minus',
  NumpadAdd: 'keypad-plus',
  NumpadEnter: 'keypad-enter',
  Numpad1: 'keypad-1',
  Numpad2: 'keypad-2',
  Numpad3: 'keypad-3',
  Numpad4: 'keypad-4',
  Numpad5: 'keypad-5',
  Numpad6: 'keypad-6',
  Numpad7: 'keypad-7',
  Numpad8: 'keypad-8',
  Numpad9: 'keypad-9',
  Numpad0: 'keypad-0',
  NumpadDecimal: 'keypad-dot',
  NumpadEqual: 'keypad-equal',
  IntlBackslash: 'nonus-backslash',
  ContextMenu: 'compose',
}

export function isModifier(code: string): boolean {
  return code in MODIFIER_BITS
}

// Reduce a set of currently-held codes into the HID report's modifier byte.
export function modifierByte(held: Iterable<string>): number {
  let bits = 0
  for (const code of held) bits |= MODIFIER_BITS[code] ?? 0
  return bits
}

// Non-modifier held codes -> usage IDs, capped at 6 (USB boot-protocol rollover).
// Returns null when over the limit so the caller can apply its overflow policy.
export function usageKeys(held: Iterable<string>): number[] | null {
  const usages: number[] = []
  for (const code of held) {
    if (isModifier(code)) continue
    const u = HID_USAGE[code]
    if (u !== undefined) usages.push(u)
  }
  return usages.length > 6 ? null : usages
}
