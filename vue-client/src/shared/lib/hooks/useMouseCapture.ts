// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

import { useConfig } from '@/core/plugins/config-plugin'

import { useContextStore } from '@/core/stores/context-store'
import { storeToRefs } from 'pinia'
import { computed, onUnmounted, ref, watch, type Ref } from 'vue'

export default function useMouseCapture(target: Ref<HTMLElement | null>, machineName: string) {
  // Target Element Event Hook
  watch(
    target,
    () => {
      // Here, we setup a basic mouseenter listener, so that we can start listening for other events when the mouse is in our component
      if (!target.value) {
        return
      }

      console.info('Starting mouseenter listener')
      target.value?.addEventListener('mouseenter', startListening)
    },
    { immediate: true },
  )

  // Build WebSocket for Mouse
  const { API_URL } = useConfig()

  const contextStore = useContextStore()
  const contextStoreRefs = storeToRefs(contextStore)

  const wss = ref<WebSocket | null>(null)

  const activeBaseUrl = computed<string | null>(() => {
    const activeContextId = contextStoreRefs.activeContext.value?.id

    if (!activeContextId || !machineName) return null

    return `${API_URL}/contexts/${activeContextId}/machines/${machineName}/usb/mouse/websocket`
  })

  watch(activeBaseUrl, () => buildWebSocket(), { immediate: true }) // Fires at start or on activeBaseUrl change

  // Cleanup
  onUnmounted(() => {
    removeScreenListeners()
    teardownWebSocket()
    // Finally, remove this event listener for when the mouse enters the component
    target.value?.removeEventListener('mouseenter', startListening)
  })

  function buildWebSocket() {
    console.log('buildWebSocket', activeBaseUrl.value)

    if (!activeBaseUrl.value) {
      console.error('No active baseUrl set!')
      return
    }

    if (wss.value) {
      teardownWebSocket()
    }

    wss.value = new WebSocket(activeBaseUrl.value)
    wss.value.binaryType = 'arraybuffer'

    // We actually only care about error messages here
    wss.value.addEventListener('error', (e) => console.error(e))
  }

  function sendMessage(msg: MouseReport) {
    msg.x = Math.round(msg.x)
    msg.y = Math.round(msg.y)

    if (!wss.value) {
      console.error('No existing web socket found')
      return
    }

    wss.value.send(JSON.stringify(msg))
  }

  function teardownWebSocket() {
    wss.value?.close()
  }

  // We fire this when we have entered the componenet
  function startListening() {
    const el = target.value
    if (!el) throw new Error('Target element for useMouseCapture not found')

    el.addEventListener('mousemove', onMouseMove)
    el.addEventListener('mousedown', onMouseDown)
  }

  function onMouseMove(e: MouseEvent) {
    const el = target.value

    if (!el) {
      throw new Error('Target element for onMouseMove not found')
    }

    const { x, y, width, height } = el.getBoundingClientRect()

    const relative_x = e.clientX - x
    const relative_y = e.clientY - y

    const x_ratio = relative_x / width
    const y_ratio = relative_y / height

    const HID_MAP_FACTOR = 0x00008000

    const x_final = x_ratio * HID_MAP_FACTOR
    const y_final = y_ratio * HID_MAP_FACTOR

    sendMessage({ buttons: [], wheel: 0, x: x_final, y: y_final } satisfies MouseReport)
  }

  function onMouseDown(e: MouseEvent) {
    const el = target.value

    if (!el) {
      throw new Error('Target element for onMouseMove not found')
    }

    const { x, y, width, height } = el.getBoundingClientRect()

    const relative_x = e.clientX - x
    const relative_y = e.clientY - y

    const x_ratio = relative_x / width
    const y_ratio = relative_y / height

    const HID_MAP_FACTOR = 0x00008000

    const x_final = x_ratio * HID_MAP_FACTOR
    const y_final = y_ratio * HID_MAP_FACTOR

    sendMessage({ buttons: [1], wheel: 0, x: x_final, y: y_final } satisfies MouseReport)
  }

  function removeScreenListeners() {
    const el = target.value

    el?.removeEventListener('mousemove', onMouseMove)
  }
}

// This matches the same interface definition in our ContextAPI Service
export interface MouseReport {
  buttons: number[]
  x: number
  y: number
  wheel: number
}
