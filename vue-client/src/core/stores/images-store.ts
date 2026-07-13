// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

import { useImageApi } from '@/shared/lib/api/images/images.api'
import type { LocalImage } from '@/shared/types/images.model'
import { defineStore } from 'pinia'
import { ref } from 'vue'

const IMAGES_LOCALSTORE_KEY = 'images_local_key'

export const useImagesStore = defineStore('images', () => {
  const { checkForExistingImageAndCreate } = useImageApi()

  const images = ref<LocalImage[]>(initializeImages())

  async function addImage(
    imageHash: string,
    image: File,
    onProgressChange: (n: number) => void,
    checkForCancellationCallback: () => boolean,
  ) {
    const { data, error } = await checkForExistingImageAndCreate(
      imageHash,
      image,
      onProgressChange,
      checkForCancellationCallback,
    )

    if (error) {
      throw new Error(`Error thrown when uploading image. ${error}`)
    }

    if (data) {
      if (images.value.find((x) => x.image_hash === imageHash)) {
        console.warn('Image already exists. Skipping upload.')
        // If the image already exists, don't add it again
        return
      }
      images.value = [...images.value, data]
      cacheImages(images.value)
    }
  }

  function deleteImage(imageHash: string) {
    images.value = images.value.filter((x) => x.image_hash !== imageHash)
    cacheImages(images.value)
  }

  function clearImageCache() {
    images.value = []
    cacheImages(images.value)
  }

  return {
    images,
    addImage,
    deleteImage,
    clearImageCache,
  }
})

function initializeImages(): LocalImage[] {
  const DEFAULT_CONFIGURATION = [] as LocalImage[]

  const configuration = localStorage.getItem(IMAGES_LOCALSTORE_KEY)

  if (!configuration) {
    return DEFAULT_CONFIGURATION
  }

  const images = JSON.parse(configuration)

  if (Array.isArray(images)) {
    return images
  }

  return DEFAULT_CONFIGURATION
}

export const cacheImages = (images: LocalImage[]) => {
  localStorage.setItem(IMAGES_LOCALSTORE_KEY, JSON.stringify(images))
}
