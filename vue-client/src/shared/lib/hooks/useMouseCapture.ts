// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

import { useConfig } from '@/core/plugins/config-plugin'
import { useContextStore } from '@/core/stores/context-store'
import { computed, shallowRef, watchEffect, type Ref } from 'vue'

// Mirrors the MouseReport in the ContextAPI service
export interface MouseReport {
  buttons: number[]
  x: number
  y: number
  wheel: number
}

type Rect = Pick<DOMRectReadOnly, 'x' | 'y' | 'width' | 'height'>

const HID_MAX = 0x7fff
const MOUSE_EVENTS = ['mousemove', 'mousedown', 'mouseup', 'mouseleave'] as const

const scale = (pos: number, origin: number, size: number) =>
  Math.round(Math.min(Math.max((pos - origin) / size, 0), 1) * HID_MAX)

// DOM bitmask (bit0 = primary) -> 1-based HID button numbers
const pressedButtons = (mask: number) =>
  [0, 1, 2, 3, 4].filter((bit) => mask & (1 << bit)).map((bit) => bit + 1)

export function toMouseReport(e: MouseEvent, rect: Rect): MouseReport {
  return {
    buttons: e.type === 'mouseleave' ? [] : pressedButtons(e.buttons),
    wheel: 0,
    x: scale(e.clientX, rect.x, rect.width),
    y: scale(e.clientY, rect.y, rect.height),
  }
}

export default function useMouseCapture(target: Ref<HTMLElement | null>, machineName: string) {
  const { API_URL } = useConfig()
  const contextStore = useContextStore()
  const socket = shallowRef<WebSocket | null>(null)
  // Set when the server closes the connection for a reason other than us
  // tearing it down ourselves (e.g. another tab already controls the mouse).
  const closeReason = shallowRef<string | null>(null)

  const url = computed(() => {
    const contextId = contextStore.activeContext?.id
    if (!contextId || !machineName) return null
    return `${API_URL}/contexts/${contextId}/machines/${machineName}/usb/mouse/websocket`
  })

  watchEffect((onCleanup) => {
    if (!url.value) return

    const ws = new WebSocket(url.value)
    ws.addEventListener('error', console.error)
    ws.addEventListener('close', (e) => {
      // Ignore events from a socket we've already superseded or torn down.
      if (socket.value !== ws) return
      socket.value = null
      if (e.code !== 1000) {
        closeReason.value = e.reason || `mouse websocket closed unexpectedly (code ${e.code})`
        console.error(closeReason.value)
      }
    })
    socket.value = ws

    onCleanup(() => {
      ws.close()
      socket.value = null
    })
  })

  watchEffect((onCleanup) => {
    const el = target.value
    if (!el) return

    const send = (e: MouseEvent) => {
      const ws = socket.value
      if (ws?.readyState !== WebSocket.OPEN) return
      ws.send(JSON.stringify(toMouseReport(e, el.getBoundingClientRect())))
    }

    MOUSE_EVENTS.forEach((type) => el.addEventListener(type, send))
    onCleanup(() => MOUSE_EVENTS.forEach((type) => el.removeEventListener(type, send)))
  })

  return { closeReason }
}
