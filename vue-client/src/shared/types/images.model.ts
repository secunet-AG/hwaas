// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

export type Images = Record<string, ImageItem>

export type LocalImage = ImageItem & {
  image_hash: string
}

export interface ImageMeta {
  sha256: string
  file_name: string
  size: number
  created: {
    secs_since_epoch: number
    nanos_since_epoch: number
  }
  architecture: string | null
  tags: string[] | null
}

export interface ImageWithMeta {
  sha256: string
  metadata: ImageMeta | null
}

// Locally persisted record of how this browser's user has interacted with an image.
export interface ImageUsage {
  uploadedAt?: number
  lastUsedAt?: number
  useCount: number
  pinned?: boolean
}

export interface ImageItem {
  file_name: string
  size: number
  created: {
    secs_since_epoch: number
    nanos_since_epoch: number
  }
}
