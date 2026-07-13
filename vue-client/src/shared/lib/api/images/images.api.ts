// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

import type { Images, ImageItem, LocalImage } from '@/shared/types/images.model'
import { useApiUrl } from '@/core/plugins/apiUrlPlugin'
import { safeFetch, type FetchResult } from '../safeFetch'
import { GetAllImagesSchema, ImageSchema } from '@/shared/schemas/images.schema'

export const useImageApi = () => {
  const { apiUrl } = useApiUrl()

  async function getAllImages(): Promise<FetchResult<Images>> {
    const res = await safeFetch(
      `${apiUrl.value}/images`,
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
      `${apiUrl.value}/images/${imageHash}`,
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
    onProgressChange: (progress: number) => void,
    checkForCancellationCallback: () => boolean,
  ): Promise<FetchResult<string>> {
    return new Promise((resolve, reject) => {
      const formData = new FormData()
      formData.append('image', image)

      const xhr = new XMLHttpRequest()
      xhr.open('POST', `${apiUrl.value}/images`)

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
    imageHash: string,
    image: File,
    onProgressChange: (n: number) => void,
    checkForCancellationCallback: () => boolean,
  ): Promise<FetchResult<LocalImage>> {
    const { data } = await getImage(imageHash)

    let final

    // If we have an image already in existence. just use this
    if (data) {
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
    const { error } = await postNewImage(image, onProgressChange, checkForCancellationCallback)

    if (error) {
      return {
        data: null,
        error: error,
      }
    }

    final = {
      size: image.size,
      created: {
        secs_since_epoch: new Date().getTime() / 1000,
        nanos_since_epoch: 0,
      },
      file_name: image.name,
      image_hash: imageHash,
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
