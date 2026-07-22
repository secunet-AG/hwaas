import { it, describe, expect } from 'vitest'
import { toTitleCase } from './titlecase'

describe('titlecase', () => {
  it('correctly splits a string', () => {
    const input = 'hello WORLD!!!'

    const result = toTitleCase(input)
    expect(result).toBe('Hello World!!!')
  })
})

describe('titlecase', () => {
  it('no split string example', () => {
    const input = 'hopefullythisworks'

    const result = toTitleCase(input)
    expect(result).toBe('Hopefullythisworks')
  })
})
