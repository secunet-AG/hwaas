// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

import type { Images, ImageItem, LocalImage } from '@/shared/types/images.model'
import { useConfig } from '@/core/plugins/config-plugin'

import { safeFetch, type FetchResult } from '../safeFetch'
import { GetAllImagesSchema, ImageSchema } from '@/shared/schemas/images.schema'

/** Compression for uploaded images, matching the `compression` query parameter of the API. */
export type ImageCompression = 'none' | 'zstd'

/**
 * We are simply checking to see if we have a `zst` or `zstd` extension,
 * the API will validate if this can actually be decompressed or not.
 */
export function detectImageCompression(file: File): ImageCompression {
  return /\.zstd?$/i.test(file.name) ? 'zstd' : 'none'
}

export const useImageApi = () => {
  const { API_URL } = useConfig()

  async function getAllImages(): Promise<FetchResult<Images>> {
    const res = await safeFetch(
      `${API_URL}/images`,
      {
        headers: {
          Accept: 'application/json',
        },
      },
      GetAllImagesSchema,
    )
    return res
  }

  async function getImage(imageHash: string): Promise<FetchResult<ImageItem>> {
    const res = await safeFetch(
      `${API_URL}/images/${imageHash}`,
      {
        headers: {
          Accept: 'application/json',
        },
      },
      ImageSchema,
    )
    return res
  }

  function postNewImage(
    image: File,
    compression: ImageCompression,
    onProgressChange: (progress: number) => void,
    checkForCancellationCallback: () => boolean,
  ): Promise<FetchResult<string>> {
    return new Promise((resolve, reject) => {
      const formData = new FormData()
      formData.append('image', image)

      const params = new URLSearchParams({ compression })

      const xhr = new XMLHttpRequest()

      xhr.open('POST', `${API_URL}/images?${params}`)

      xhr.upload.onprogress = (event) => {
        if (checkForCancellationCallback()) {
          // We pass in a callback that aborts the request if cancelled.
          xhr.abort()
        }
        if (event.lengthComputable) {
          const percentComplete = Math.round((event.loaded / event.total) * 100)
          onProgressChange(percentComplete)
        }
      }

      xhr.onload = () => {
        if (xhr.status >= 200 && xhr.status < 300) {
          resolve({ data: xhr.responseText, error: null })
        } else {
          reject(new Error(`Upload failed with status ${xhr.status}`))
        }
      }

      xhr.onerror = () => reject(new Error('Network error during upload'))

      xhr.send(formData)
    })
  }

  async function checkForExistingImageAndCreate(
    imageHash: string | null,
    image: File,
    compression: ImageCompression,
    onProgressChange: (n: number) => void,
    checkForCancellationCallback: () => boolean,
  ): Promise<FetchResult<LocalImage>> {
    const { data } = imageHash ? await getImage(imageHash) : { data: null }

    let final

    // If we have an image already in existence. just use this
    if (data && imageHash) {
      return {
        data: {
          size: data.size,
          created: {
            secs_since_epoch: new Date().getTime() / 1000,
            nanos_since_epoch: 0,
          },
          file_name: data.file_name,
          image_hash: imageHash,
        } as LocalImage,
        error: null,
      }
    }
    // Otherwise, we post the new image. We also pass in a callback for progress and cancellation.
    const { data: uploadedHash, error } = await postNewImage(
      image,
      compression,
      onProgressChange,
      checkForCancellationCallback,
    )

    if (error || !uploadedHash) {
      return {
        data: null,
        error: error ?? 'Upload did not return an image hash',
      }
    }

    if (compression !== 'none') {
      // The local file size and hash refer to the compressed file, so ask the server for the
      // metadata of the decompressed image it actually stored.
      const { data: stored, error: metadataError } = await getImage(uploadedHash)

      if (metadataError) {
        return { data: null, error: metadataError }
      }

      return {
        data: { ...stored, image_hash: uploadedHash } as LocalImage,
        error: null,
      }
    }

    final = {
      size: image.size,
      created: {
        secs_since_epoch: new Date().getTime() / 1000,
        nanos_since_epoch: 0,
      },
      file_name: image.name,
      image_hash: imageHash ?? uploadedHash,
    } as LocalImage

    return {
      data: final,
      error: null,
    }
  }

  return {
    getImage,
    getAllImages,
    postNewImage,
    checkForExistingImageAndCreate,
  }
}
