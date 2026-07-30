<!--
SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>

SPDX-License-Identifier: Apache-2.0
-->

<script setup lang="ts">
import { CODE_TO_KEY, isModifier, modifierByte } from '@/shared/lib/hooks/useKeyboardCapture'

export interface ComboEvent {
  keys: string[]
  modifier: number
}

// The sticky modifier set is owned by the parent and shared with the physical
// keyboard capture, so a toggled modifier applies to the next real keypress.
const props = defineProps<{
  stickyModifiers: Set<string>
}>()

const emit = defineEmits<{
  send: [combo: ComboEvent]
}>()

interface KeyDef {
  label: string
  codes: string[]
}

const modifierKeys: KeyDef[] = [
  { label: 'Ctrl', codes: ['ControlLeft'] },
  { label: 'Alt', codes: ['AltLeft'] },
  { label: 'Shift', codes: ['ShiftLeft'] },
  { label: 'Super', codes: ['MetaLeft'] },
]

const functionKeys: KeyDef[] = Array.from({ length: 12 }, (_, i) => ({
  label: `F${i + 1}`,
  codes: [`F${i + 1}`],
}))

// A list of keys that cannot be easily sent due to lack of crossplatform web API for capture
//
// https://developer.mozilla.org/en-US/docs/Web/API/Keyboard/lock
const commonKeys: KeyDef[] = [
  { label: 'Esc', codes: ['Escape'] },
  { label: 'Tab', codes: ['Tab'] },
  { label: 'Del', codes: ['Delete'] },
  { label: 'Enter', codes: ['Enter'] },
  { label: 'PrtSc', codes: ['PrintScreen'] },
  { label: 'Ins', codes: ['Insert'] },
  { label: 'Home', codes: ['Home'] },
  { label: 'End', codes: ['End'] },
  { label: 'PgUp', codes: ['PageUp'] },
  { label: 'PgDn', codes: ['PageDown'] },
]

// Common groupings of keys that cannot be captured due to lack of web support
//
// https://developer.mozilla.org/en-US/docs/Web/API/Keyboard/lock
const combos: KeyDef[] = [
  { label: 'Ctrl+Alt+Del', codes: ['ControlLeft', 'AltLeft', 'Delete'] },
  { label: 'Alt+Tab', codes: ['AltLeft', 'Tab'] },
  { label: 'Alt+F4', codes: ['AltLeft', 'F4'] },
  { label: 'Ctrl+Shift+Esc', codes: ['ControlLeft', 'ShiftLeft', 'Escape'] },
  { label: 'Ctrl+Alt+F1', codes: ['ControlLeft', 'AltLeft', 'F1'] },
  { label: 'Ctrl+Alt+F2', codes: ['ControlLeft', 'AltLeft', 'F2'] },
  { label: 'Alt+F2', codes: ['AltLeft', 'F2'] },
]

// See https://docs.kernel.org/admin-guide/sysrq.html
//
// https://developer.mozilla.org/en-US/docs/Web/API/Keyboard/lock
const sysRqCombos: KeyDef[] = [
  { label: 'Unraw (R)', codes: ['AltLeft', 'PrintScreen', 'KeyR'] },
  { label: 'Term (E)', codes: ['AltLeft', 'PrintScreen', 'KeyE'] },
  { label: 'Kill (I)', codes: ['AltLeft', 'PrintScreen', 'KeyI'] },
  { label: 'Sync (S)', codes: ['AltLeft', 'PrintScreen', 'KeyS'] },
  { label: 'Unmount (U)', codes: ['AltLeft', 'PrintScreen', 'KeyU'] },
  { label: 'Reboot (B)', codes: ['AltLeft', 'PrintScreen', 'KeyB'] },
  { label: 'Poweroff (O)', codes: ['AltLeft', 'PrintScreen', 'KeyO'] },
  { label: 'Full oom (F)', codes: ['AltLeft', 'PrintScreen', 'KeyF'] },
  { label: 'SAK (K)', codes: ['AltLeft', 'PrintScreen', 'KeyK'] },
  { label: 'Crash (C)', codes: ['AltLeft', 'PrintScreen', 'KeyC'] },
]

function toggleModifier(code: string) {
  if (props.stickyModifiers.has(code)) props.stickyModifiers.delete(code)
  else props.stickyModifiers.add(code)
}

// Build a report from the clicked codes, then clear sticky state
function press(codes: string[]) {
  const all = [...props.stickyModifiers, ...codes]
  const keys: string[] = []
  for (const code of all) {
    if (isModifier(code)) continue
    const name = CODE_TO_KEY[code]
    if (name !== undefined) keys.push(name)
  }
  emit('send', { keys, modifier: modifierByte(all) })
  props.stickyModifiers.clear()
}

const buttonClass =
  'rounded-md border border-(--app-secondary-text)/30 px-2 py-1 text-sm ' +
  'text-(--app-secondary-text) hover:bg-(--app-secondary-text)/10 cursor-pointer'
</script>

<template>
  <div
    class="divide-(--app-secondary-text)/20 bg-(--app-bg)/70 absolute top-16 right-3 z-10 flex max-h-[calc(100%-5rem)] w-56 flex-col divide-y overflow-y-auto rounded-xl backdrop-blur-[4px] select-none"
  >
    <div class="flex flex-col gap-1.5 p-3">
      <span class="text-(--app-secondary-text)/70 text-xs">Modifiers</span>
      <div class="flex flex-wrap gap-1.5">
        <button
          v-for="key in modifierKeys"
          :key="key.label"
          :class="[
            buttonClass,
            stickyModifiers.has(key.codes[0]) ? 'bg-(--app-secondary-text)/20' : '',
          ]"
          @click="toggleModifier(key.codes[0])"
        >
          {{ key.label }}
        </button>
      </div>
    </div>

    <div class="flex flex-col gap-1.5 p-3">
      <span class="text-(--app-secondary-text)/70 text-xs">Function</span>
      <div class="flex flex-wrap gap-1.5">
        <button
          v-for="key in functionKeys"
          :key="key.label"
          :class="buttonClass"
          @click="press(key.codes)"
        >
          {{ key.label }}
        </button>
      </div>
    </div>

    <div class="flex flex-col gap-1.5 p-3">
      <span class="text-(--app-secondary-text)/70 text-xs">Keys</span>
      <div class="flex flex-wrap gap-1.5">
        <button
          v-for="key in commonKeys"
          :key="key.label"
          :class="buttonClass"
          @click="press(key.codes)"
        >
          {{ key.label }}
        </button>
      </div>
    </div>

    <div class="flex flex-col gap-1.5 p-3">
      <span class="text-(--app-secondary-text)/70 text-xs">Combos</span>
      <div class="flex flex-wrap gap-1.5">
        <button
          v-for="combo in combos"
          :key="combo.label"
          :class="buttonClass"
          @click="press(combo.codes)"
        >
          {{ combo.label }}
        </button>
      </div>
    </div>

    <details class="group flex flex-col p-3">
      <summary
        class="text-(--app-secondary-text)/70 flex cursor-pointer items-center justify-between text-xs"
        title="These invoke kernel Magic SysRq handlers and only work if SysRq is enabled on the target (e.g. kernel.sysrq=1 via sysctl or CONFIG_MAGIC_SYSRQ)."
      >
        <span>Magic SysRq</span>
        <span class="transition-transform group-open:rotate-90">&rsaquo;</span>
      </summary>
      <p class="text-(--app-secondary-text)/50 mt-1.5 text-[0.7rem] leading-snug">
        Requires SysRq enabled on the target kernel(<code>kernel.sysrq=1</code>).
      </p>
      <div class="mt-1.5 flex flex-wrap gap-1.5">
        <button
          v-for="combo in sysRqCombos"
          :key="combo.label"
          :class="buttonClass"
          @click="press(combo.codes)"
        >
          {{ combo.label }}
        </button>
      </div>
    </details>
  </div>
</template>
