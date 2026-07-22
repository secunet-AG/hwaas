import { describe, it, expect } from 'vitest'
import { deserializeContextConfig } from './context-validation'

describe('deserializeContextConfig', () => {
  it('parses a valid config correctly', () => {
    const input = JSON.stringify({
      machines: {
        box1: { machine_id: 1, platform: 'intel-nuc-12' },
      },
    })

    const result = deserializeContextConfig(input)
    expect(result.machines.box1.machine_id).toBe(1)
  })
  ;(it('parses a valid multi item string config correctly (API example)', () => {
    const input = `
    {
        "machines": {
                "additionalProp1": {
                "machine_id": 0,
                "platform": "string"
            },
                "additionalProp2": {
                "machine_id": 1,
                "platform": "string"
            },
                "additionalProp3": {
                "machine_id": 2,
                "platform": "string"
            }
        }
    }
    `
    const result = deserializeContextConfig(input)
    expect(result.machines.additionalProp1.machine_id).toBe(0)
    expect(result.machines.additionalProp2.machine_id).toBe(1)
    expect(result.machines.additionalProp3.machine_id).toBe(2)

    expect(result.machines.additionalProp1.platform).toBe('string')
    expect(result.machines.additionalProp2.platform).toBe('string')
    expect(result.machines.additionalProp3.platform).toBe('string')
  }),
    it('throws on invalid JSON', () => {
      expect(() => deserializeContextConfig('{bad json')).toThrow(SyntaxError)
    }))

  it('throws on array input', () => {
    const input = JSON.stringify([{ a: 1 }])
    expect(() => deserializeContextConfig(input)).toThrow(
      'Invalid configuration shape. Must be a valid, none null, none array like JSON object',
    )
  })

  it('throws on null input', () => {
    const input = 'null'
    expect(() => deserializeContextConfig(input)).toThrow(
      'Cannot convert undefined or null to object',
    )
  })

  it('throws when there is more than one top-level key', () => {
    const input = JSON.stringify({
      machines: {},
      extra: {},
    })
    expect(() => deserializeContextConfig(input)).toThrow(
      'Invalid configuration file, additional properties found',
    )
  })

  it('throws when "machines" is missing', () => {
    const input = JSON.stringify({ notMachines: {} })
    expect(() => deserializeContextConfig(input)).toThrow('Machines not found')
  })

  it('throws when a machine has more than 2 properties', () => {
    const input = JSON.stringify({
      machines: {
        box1: {
          machine_id: 1,
          platform: 'intel-nuc-12',
          extra: 'bad',
        },
      },
    })
    expect(() => deserializeContextConfig(input)).toThrow(
      'Invalid configuration file, additional properties found on machines',
    )
  })

  it('allows missing machine_id', () => {
    const input = JSON.stringify({
      machines: {
        box1: { platform: 'intel-nuc-12' },
      },
    })
    const result = deserializeContextConfig(input)
    expect(result.machines.box1.platform).toBe('intel-nuc-12')
    expect(result.machines.box1.machine_id).toBe(undefined)
  })

  it('throws when platform is missing', () => {
    const input = JSON.stringify({
      machines: {
        box1: { machine_id: 1 },
      },
    })
    expect(() => deserializeContextConfig(input)).toThrow(
      'Invalid configuration file, machine missing correct information!',
    )
  })

  it('throws when machine_id is not a number', () => {
    const input = JSON.stringify({
      machines: {
        box1: { machine_id: '1', platform: 'intel-nuc-12' },
      },
    })
    expect(() => deserializeContextConfig(input)).toThrow(
      'Invalid configuration file, machine missing correct information!',
    )
  })

  it('throws when platform is not a string', () => {
    const input = JSON.stringify({
      machines: {
        box1: { machine_id: 1, platform: 123 },
      },
    })
    expect(() => deserializeContextConfig(input)).toThrow(
      'Invalid configuration file, machine missing correct information!',
    )
  })

  it('accepts machine_id equal to 0', () => {
    const input = JSON.stringify({
      machines: {
        box1: { machine_id: 0, platform: 'intel-nuc-12' },
      },
    })
    const result = deserializeContextConfig(input)
    expect(result.machines.box1.machine_id).toBe(0)
  })
})
