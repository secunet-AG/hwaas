// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

import { useApiUrl } from '@/core/plugins/apiUrlPlugin'
import { useContextStore } from '@/core/stores/context-store'
import { storeToRefs } from 'pinia'
import { computed, onUnmounted, ref, watch } from 'vue'
import type { KeyboardReport } from './useKeyboardCapture'

export function useKeyboardWebsocket() {
  const { apiUrl } = useApiUrl()

  const contextStore = useContextStore()
  const contextStoreRefs = storeToRefs(contextStore)

  const activeMachineName = ref<string | null>(null)

  const wss = ref<WebSocket | null>(null)

  const setActiveMachineName = (machineName: string) => {
    activeMachineName.value = machineName
  }

  const activeBaseUrl = computed<string | null>(() => {
    const activeContextId = contextStoreRefs.activeContext.value?.id

    if (!activeContextId || !activeMachineName.value) return null

    return `${apiUrl.value}/contexts/${activeContextId}/machines/${activeMachineName.value}/usb/keyboard/websocket`
  })

  // Lifecycle hooks

  watch(activeBaseUrl, () => buildWebSocket(), { immediate: true }) // Fires at start or on activeBaseUrl change

  onUnmounted(() => unsubscribe())

  function buildWebSocket() {
    console.log('buildWebSocket')

    if (wss.value) {
      unsubscribe()
    }

    if (!activeBaseUrl.value) {
      console.error('No active baseUrl set!')
      return
    }

    wss.value = new WebSocket(activeBaseUrl.value)
    wss.value.binaryType = 'arraybuffer'

    // We actually only care about error messages here, as there is no useful information to receive
    wss.value.addEventListener('error', (e) => console.error(e))
  }

  function sendMessage(msg: KeyboardReport) {
    console.log('sendMessage', msg)
    if (!wss.value) {
      console.error('No existing web socket found')
      return
    }
    wss.value.send(JSON.stringify(msg))
  }

  function unsubscribe() {
    wss.value?.close()
  }

  return {
    sendMessage,
    setActiveMachineAndPort: setActiveMachineName,
    unsubscribe,
  }
}
