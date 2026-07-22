import z from 'zod'

export const MachinePowerResponseSchema = z.array(
  z.object({
    power_id: z.string(),
    state: z.boolean(),
  }),
)

export const SerialResponseSchema = z.array(z.string())

export const NetworkInterfacesSchema = z.array(z.string())
