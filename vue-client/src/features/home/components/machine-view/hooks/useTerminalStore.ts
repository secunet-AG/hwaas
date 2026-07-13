// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

import { defineStore } from 'pinia'
import { ref } from 'vue'

// In TypeScript, we can't implement a custom hash implementation afaik,
// and tuple types [1, 'example'] check for the same reference for Map keys
// rather than the value. So we will serialize our contextid-machinename
export type ContextMachineKey = string
export type TerminalHistory = string
export type MachineTerminalCache = Map<ContextMachineKey, TerminalHistory>

function buildTerminalHashKey(contextId: string, machineName: string): ContextMachineKey {
  return `${contextId}-${machineName}`
}

export const useTerminalStore = defineStore('terminalHistory', () => {
  const cachedTerminalHistory = ref<MachineTerminalCache>(new Map())

  const getHistoryForTerminal = (contextId: string, machineName: string): string => {
    const terminalHistoryKey = buildTerminalHashKey(contextId, machineName)
    return cachedTerminalHistory.value.get(terminalHistoryKey) ?? ''
  }

  const setHistory = (contextId: string, machineName: string, history: string) => {
    const terminalHistoryKey = buildTerminalHashKey(contextId, machineName)
    cachedTerminalHistory.value.set(terminalHistoryKey, history)
  }

  return {
    setHistory,
    getHistoryForTerminal,
  }
})
