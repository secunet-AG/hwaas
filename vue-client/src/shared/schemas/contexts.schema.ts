// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

import z from 'zod'

export const ContextReservationSchema = z.string()

export const ContextLifetimeSchema = z.object({ lifetime: z.number().positive() })

export const ContextMachineSchema = z.object({
  id: z.number(),
  platform: z.string(),
})

export const ContextMachinesSchema = z.array(z.string())
