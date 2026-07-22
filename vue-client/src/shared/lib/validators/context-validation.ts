export type ContextConfig = { machines: Record<string, { machine_id: number; platform: string }> }

// Our data shape looks like the below:

// {
//   "machines": {
//     "box1": {
//       "machine_id": 1,
//       "platform": "intel-nuc-12"
//     },
//     "box2": {
//       "machine_id": 2,
//       "platform": "intel-nuc-12"
//     },
//     "box3": {
//       "machine_id": 3,
//       "platform": "i-am-an-intel-nuc-but-special"
//     }
//   }
// }

export function deserializeContextConfig(config: string): ContextConfig {
  const parsed = JSON.parse(config)

  if (typeof parsed !== 'object' || typeof parsed === null || Array.isArray(parsed)) {
    throw new Error(
      'Invalid configuration shape. Must be a valid, none null, none array like JSON object',
    )
  }

  // To determine if it's valid, we will look through the keys, and make sure machine_id and platform are present on each key

  if (Object.keys(parsed).length > 1) {
    throw new Error('Invalid configuration file, additional properties found')
  }

  const { machines } = parsed

  if (!machines) throw new Error('Machines not found')

  Object.keys(machines).forEach((k) => {
    if (Object.keys(machines[k]).length > 2) {
      throw new Error('Invalid configuration file, additional properties found on machines')
    }
    const { machine_id, platform } = machines[k]
    // Nasty edge case of ID zero machine, so we have to check for null and undefined
    if (
      !platform ||
      (machine_id !== undefined && machine_id !== null && typeof machine_id !== 'number') ||
      typeof platform !== 'string'
    ) {
      throw new Error('Invalid configuration file, machine missing correct information!')
    }
  })

  return parsed as ContextConfig
}
