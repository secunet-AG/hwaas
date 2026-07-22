// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

type DurationToTimeMap = Record<AlertDuration, number>

const durationToTimeMap = {
  short: 1200,
  medium: 3200,
  long: 7200,
} as DurationToTimeMap

export type AlertDuration = 'short' | 'medium' | 'long'

export type AlertSeverity = 'success' | 'warning' | 'error'

export interface ToasterAlertOptions {
  duration: AlertDuration
  severity: AlertSeverity
  message?: string
}

const defaultToasterOptions = {
  duration: 'medium',
  severity: 'error',
} as ToasterAlertOptions

export interface ToasterAlert {
  title: string
  message: string
  alertSeverity: AlertSeverity
}

export interface ToasterAlertDisplay extends Omit<ToasterAlert, 'duration'> {
  durationTime: number
}

export function alertBuilder(title: string, options: ToasterAlertOptions): ToasterAlertDisplay {
  const durationTime = durationToTimeMap[options.duration]
  return {
    title: title,
    durationTime: durationTime,
    message: options.message ?? '',
    alertSeverity: options.severity,
  }
}
