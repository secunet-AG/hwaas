// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

import z from 'zod'

export const MachinePowerResponseSchema = z.array(
  z.object({
    power_id: z.string(),
    state: z.boolean(),
  }),
)

export const SerialResponseSchema = z.array(z.string())

export const NetworkInterfacesSchema = z.array(z.string())
