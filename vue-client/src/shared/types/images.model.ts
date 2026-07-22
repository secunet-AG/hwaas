// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

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
