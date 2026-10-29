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


export const ImageWithMetaSchema = z.object({
  sha256: z.string(),
  metadata:  z.nullable(z.object({
    sha256: z.string(),
    file_name: z.string(),
    size: z.number().positive(),
    created: z.object({
      secs_since_epoch: z.number(),
      nanos_since_epoch: z.number(),
    }),
    architecture: z.nullable(z.string()),
    tags: z.nullable(z.array(z.string()))
  }))
})

export const GetAllImagesWithMetaSchema = z.array(ImageWithMetaSchema)
