// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

import { useConfig } from '@/core/plugins/config-plugin'

import { useContextStore } from '@/core/stores/context-store'
import { storeToRefs } from 'pinia'
import { computed, onUnmounted, ref, watch } from 'vue'
import type { KeyboardReport } from './useKeyboardCapture'

export function useKeyboardWebsocket() {
  const { API_URL } = useConfig()

  const contextStore = useContextStore()
  const contextStoreRefs = storeToRefs(contextStore)

  const activeMachineName = ref<string | null>(null)

  const wss = ref<WebSocket | null>(null)
  // Set when the server closes the connection for a reason other than us
  // tearing it down ourselves (e.g. another tab already controls the keyboard).
  const closeReason = ref<string | null>(null)

  const setActiveMachineName = (machineName: string) => {
    activeMachineName.value = machineName
  }

  const activeBaseUrl = computed<string | null>(() => {
    const activeContextId = contextStoreRefs.activeContext.value?.id

    if (!activeContextId || !activeMachineName.value) return null

    return `${API_URL}/contexts/${activeContextId}/machines/${activeMachineName.value}/usb/keyboard/websocket`
  })

  // Lifecycle hooks

  watch(activeBaseUrl, () => buildWebSocket())

  onUnmounted(() => unsubscribe())

  function buildWebSocket() {
    if (wss.value) {
      unsubscribe()
    }

    if (!activeBaseUrl.value) {
      return
    }

    const ws = new WebSocket(activeBaseUrl.value)
    ws.binaryType = 'arraybuffer'

    ws.addEventListener('error', (e) => console.error(e))
    ws.addEventListener('close', (e) => {
      // Ignore events from a socket we've already superseded or torn down.
      if (wss.value !== ws) return
      wss.value = null
      if (e.code !== 1000) {
        closeReason.value = e.reason || `keyboard websocket closed unexpectedly (code ${e.code})`
        console.error(closeReason.value)
      }
    })

    wss.value = ws
  }

  function sendMessage(msg: KeyboardReport) {
    if (!wss.value || wss.value.readyState !== WebSocket.OPEN) {
      console.warn('No open web socket found')
      return
    }
    wss.value.send(JSON.stringify(msg))
  }

  function unsubscribe() {
    wss.value?.close()
    wss.value = null
  }

  return {
    sendMessage,
    setActiveMachineAndPort: setActiveMachineName,
    unsubscribe,
    closeReason,
  }
}
