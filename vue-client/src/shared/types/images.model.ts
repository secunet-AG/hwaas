export type Images = Record<string, ImageItem>

export type LocalImage = ImageItem & {
  image_hash: string
}

export interface ImageItem {
  file_name: string
  size: number
  created: {
    secs_since_epoch: number
    nanos_since_epoch: number
  }
}
