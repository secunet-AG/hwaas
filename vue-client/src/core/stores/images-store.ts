// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

import { useImageApi } from '@/shared/lib/api/images/images.api'
import type { ImageUsage, ImageWithMeta } from '@/shared/types/images.model'
import { useAsyncState, useLocalStorage } from '@vueuse/core'
import { defineStore } from 'pinia'
import { computed, watch } from 'vue'

const IMAGE_USAGE_LOCALSTORE_KEY = 'image_usage_local_key'

// Usage entries for images that disappear from the server are dropped after this long,
// so a briefly unavailable image doesn't immediately lose its history.
const STALE_USAGE_MS = 1000 * 3600 * 24 * 30

export const useImagesStore = defineStore('images', () => {
  const { checkForExistingImageAndCreate, getAllImages } = useImageApi()

  const {
    state: images,
    isLoading,
    error,
    execute: refreshImages,
  } = useAsyncState(
    async () => {
      const res = await getAllImages()
      if (res.error || !res.data) throw res.error ?? new Error('No image data')
      return res.data
    },
    [],
    { immediate: true, resetOnExecute: false },
  )

  // Per-browser history of how the user interacts with images. The server owns the image
  // list; this only influences ordering and highlighting.
  const usage = useLocalStorage<Record<string, ImageUsage>>(IMAGE_USAGE_LOCALSTORE_KEY, {})

  function touchUsage(sha256: string, patch: Partial<ImageUsage>) {
    const prev = usage.value[sha256] ?? { useCount: 0 }
    usage.value = { ...usage.value, [sha256]: { ...prev, ...patch } }
  }

  function recordUpload(sha256: string) {
    touchUsage(sha256, { uploadedAt: Date.now() })
  }

  function recordSelection(sha256: string) {
    const prev = usage.value[sha256]
    touchUsage(sha256, { lastUsedAt: Date.now(), useCount: (prev?.useCount ?? 0) + 1 })
  }

  function togglePinned(sha256: string) {
    touchUsage(sha256, { pinned: !usage.value[sha256]?.pinned })
  }

  function forgetUsage(sha256: string) {
    usage.value = Object.fromEntries(Object.entries(usage.value).filter(([h]) => h !== sha256))
  }

  function lastInteraction(u: ImageUsage | undefined) {
    return Math.max(u?.lastUsedAt ?? 0, u?.uploadedAt ?? 0)
  }

  // Pinned first, then most recently uploaded/used by this user, then newest on the server.
  const sortedImages = computed<ImageWithMeta[]>(() =>
    [...images.value].sort((a, b) => {
      const ua = usage.value[a.sha256]
      const ub = usage.value[b.sha256]
      return (
        Number(!!ub?.pinned) - Number(!!ua?.pinned) ||
        lastInteraction(ub) - lastInteraction(ua) ||
        (b.metadata?.created.secs_since_epoch ?? 0) - (a.metadata?.created.secs_since_epoch ?? 0)
      )
    }),
  )

  // Drop history for images the server no longer has, once they've been gone a while.
  watch(images, (list) => {
    if (!list.length) return
    const known = new Set(list.map((x) => x.sha256))
    const cutoff = Date.now() - STALE_USAGE_MS
    const kept = Object.entries(usage.value).filter(
      ([hash, u]) => known.has(hash) || u.pinned || lastInteraction(u) > cutoff,
    )
    if (kept.length !== Object.keys(usage.value).length) {
      usage.value = Object.fromEntries(kept)
    }
  })

  async function addImage(
    imageHash: string,
    image: File,
    onProgressChange: (n: number) => void,
    checkForCancellationCallback: () => boolean,
  ) {
    const { error } = await checkForExistingImageAndCreate(
      imageHash,
      image,
      onProgressChange,
      checkForCancellationCallback,
    )

    if (error) throw new Error(`Error thrown when uploading image. ${error}`)

    recordUpload(imageHash)
    await refreshImages()
  }

  return {
    images,
    sortedImages,
    usage,
    isLoading,
    error,
    addImage,
    refreshImages,
    recordSelection,
    togglePinned,
    forgetUsage,
  }
})
