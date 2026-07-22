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
