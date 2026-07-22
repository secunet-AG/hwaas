// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

import { parseLifetime, secondsToReadable } from './time-validation'
import { expect, it, describe } from 'vitest'

describe('serialize a time from the example', () => {
  it('parses a time correctly', () => {
    const exampeOne = '1h 30m'
    const exampleTwo = '23s 10m 3h'
    const exampleThree = '90m'
    const exampleFour = '1h'
    const exampleFive = '1h  30s'

    expect(parseLifetime(exampeOne)).toBe(3600 + 30 * 60)
    expect(parseLifetime(exampleTwo)).toBe(23 + 60 * 10 + 3600 * 3)
    expect(parseLifetime(exampleThree)).toBe(60 * 90)
    expect(parseLifetime(exampleFour)).toBe(3600)
    expect(parseLifetime(exampleFive)).toBe(3630)
  })
  it('should not be able to parse this', () => {
    expect(() => parseLifetime('abc')).toThrow('Invalid Unit')
    expect(() => parseLifetime('23s 10d')).toThrow('Invalid Unit')
    expect(() => parseLifetime('abcs xyh')).toThrow('Could not parse scalar')
  })
})

describe('print a lifetime cleanly', () => {
  it('can print a lifetime', () => {
    const exampleOne = 3600
    const exampleTwo = 7205
    const exampleThree = 3600 * 3 + 60 * 2 + 2
    const exampleFour = 5

    expect(secondsToReadable(exampleOne)).toBe('1 hour remaining.')
    expect(secondsToReadable(exampleTwo)).toBe('2 hours 5 seconds remaining.')
    expect(secondsToReadable(exampleThree)).toBe('3 hours 2 minutes 2 seconds remaining.')
    expect(secondsToReadable(exampleFour)).toBe('5 seconds remaining.')
  })
})
