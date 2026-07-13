// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

export const parseLifetime = (input: string): number => {
  const terms = input
    .trim()
    .split(' ')
    .filter((x) => x !== '')

  let total = 0

  const unitToMult = {
    s: 1,
    m: 60,
    h: 3600,
  }

  terms.forEach((x) => {
    const unit = x[x.length - 1]
    if (!unit) throw new Error('Missing unit')

    if (!['s', 'm', 'h'].includes(unit)) {
      throw new Error('Invalid Unit')
    }

    const scalar = Number(x.slice(0, x.length - 1))
    if (Number.isNaN(scalar)) throw new Error('Could not parse scalar')
    const parsed = Number(scalar)
    total += parsed * unitToMult[unit as 's' | 'm' | 'h']
  })

  return total
}

export const secondsToReadable = (input: number): string => {
  const hours = Math.floor(input / 3600)
  const minutes = Math.floor((input - hours * 3600) / 60)
  const seconds = input - hours * 3600 - minutes * 60

  const hoursMsg = hours > 0 ? `${hours} ${hours === 1 ? 'hour' : 'hours'}` : null
  const minutesMsg = minutes > 0 ? `${minutes} ${minutes === 1 ? 'minute' : 'minutes'}` : null
  const secondsMsg = seconds > 0 ? `${seconds} ${seconds === 1 ? 'second' : 'seconds'}` : null

  return [hoursMsg, minutesMsg, secondsMsg].filter((x) => !!x).join(' ') + ' remaining.'
}
