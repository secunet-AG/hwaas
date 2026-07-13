// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

import z from 'zod'

export const ImageSchema = z.object({
  file_name: z.string(),
  size: z.number().positive(),
  created: z.object({
    secs_since_epoch: z.number(),
    nanos_since_epoch: z.number(),
  }),
})

export const GetAllImagesSchema = z.record(z.string(), ImageSchema)
