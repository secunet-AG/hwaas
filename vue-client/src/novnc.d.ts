// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

declare module '@novnc/novnc/lib/rfb.js' {
  export interface RfbEvent {
    detail: {
      clean: boolean
      reason: string
      error: {
        message: string
      }
      message: string
    }
  }

  export interface RfbDisplay {
    autoscale(): void
    _scale: number
  }

  export interface RFBOptions {
    credentials?: { username?: string; password?: string }
    clipViewport?: boolean
    scaleViewport?: boolean
    shared?: boolean
    resizeSession?: boolean
    viewOnly?: boolean
    [key: string]: unknown
  }

  export default class RFB {
    _display: RfbDisplay
    resizeSession: boolean
    scaleViewport: boolean
    constructor(target: HTMLElement, url: string, options?: RFBOptions)

    addEventListener(event: string, listener: (e: RfbEvent) => void): void
    removeEventListener(event: string, listener: (e: RfbEvent) => void): void
    disconnect(): void
    viewportChange(): void
  }
}
