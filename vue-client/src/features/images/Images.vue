<!--
SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>

SPDX-License-Identifier: Apache-2.0
-->

<script setup lang="ts">
import UploadImageModal from './components/UploadImageModal.vue'
import { computed, ref } from 'vue'
import Searchbar from '@/shared/ui/Searchbar.vue'
import { bytesToSize } from '@/shared/lib/etc/bytes-to-size'
import { useImagesStore } from '@/core/stores/images-store'
import FadeInOut from '@/shared/ui/transitions/FadeInOut.vue'
import { storeToRefs } from 'pinia'
import type { ImageMeta } from '@/shared/types/images.model'

const imageStore = useImagesStore()

const { sortedImages, usage, isLoading, error } = storeToRefs(imageStore)

const searchValue = ref('')

// Note(Luke): We simply filter on the client side here, this could be query based
// at some point with pagination but this works for our scale.
//
// We search for a string inclusion for the sha256 hash, tags, and file name, normalizing
// to lower case beforehand..
const filtered = computed(() => {
  const query = searchValue.value.toLowerCase()
  return sortedImages.value.filter(
    (x) =>
      x.sha256.includes(query) ||
      (x.metadata?.file_name ?? '').toLowerCase().includes(query) ||
      (x.metadata?.tags ?? []).some((t) => t.toLowerCase().includes(query)),
  )
})

/// We are logging image usage in local storage, we do this to rank more "relevant" images without server side state
function usageLabel(hash: string) {
  const u = usage.value[hash]
  if (!u) return null
  if (u.lastUsedAt)
    return `last used ${new Date(u.lastUsedAt).toLocaleDateString('de-DE')}`
  if (u.uploadedAt) return 'Uploaded by you'
  return null
}

function copyIdToClipboard(e: MouseEvent, hash: string) {
  e.stopImmediatePropagation() // Prevent this from bubbling to search bar below
  navigator.clipboard.writeText(hash)
}

function formatCreated(meta: ImageMeta | null) {
  return meta ? new Date(meta.created.secs_since_epoch * 1000).toLocaleDateString('de-DE') : '-'
}

function formatSize(meta: ImageMeta | null) {
  return meta ? bytesToSize(meta.size) : '-'
}
</script>

<template>
  <div class="flex h-full w-full flex-col">
    <div class="flex w-full items-center justify-between">
      <h2 class="text-2xl">Images</h2>
      <div class="flex gap-6">
        <Searchbar @search-event="(x) => (searchValue = x)" />
        <UploadImageModal />
      </div>
    </div>
    <FadeInOut>
      <div v-if="filtered.length" class="mt-6 flex h-full flex-col gap-3 overflow-y-auto">
        <div
          class="app-card flex w-full items-center justify-between overflow-clip"
          v-for="item of filtered"
          :key="item.sha256"
        >
          <div class="flex w-full flex-col gap-1">
            <div
              class="border-(--app-secondary-border) flex w-full items-center gap-3 border-b pb-3"
            >
              <div class="flex flex-nowrap items-center gap-1 text-ellipsis">
                <h6 class="text-ellipsis text-nowrap text-lg">
                  {{ item.metadata?.file_name ?? 'Unknown image' }} -
                </h6>
                <span
                  class="text-(--app-secondary-text) text-ellipsis text-nowrap pt-0.75 text-sm"
                  >{{ item.sha256 }}</span
                >

                <button
                  class="flex cursor-pointer items-center gap-1 transition duration-300 hover:-translate-y-0.5 hover:brightness-200"
                  v-on:click="(e) => copyIdToClipboard(e, item.sha256)"
                >
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke-width="1.0"
                    stroke="currentColor"
                    class="text-(--app-secondary-text) h-4 w-4"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      d="M8.25 7.5V6.108c0-1.135.845-2.098 1.976-2.192.373-.03.748-.057 1.123-.08M15.75 18H18a2.25 2.25 0 0 0 2.25-2.25V6.108c0-1.135-.845-2.098-1.976-2.192a48.424 48.424 0 0 0-1.123-.08M15.75 18.75v-1.875a3.375 3.375 0 0 0-3.375-3.375h-1.5a1.125 1.125 0 0 1-1.125-1.125v-1.5A3.375 3.375 0 0 0 6.375 7.5H5.25m11.9-3.664A2.251 2.251 0 0 0 15 2.25h-1.5a2.251 2.251 0 0 0-2.15 1.586m5.8 0c.065.21.1.433.1.664v.75h-6V4.5c0-.231.035-.454.1-.664M6.75 7.5H4.875c-.621 0-1.125.504-1.125 1.125v12c0 .621.504 1.125 1.125 1.125h9.75c.621 0 1.125-.504 1.125-1.125V16.5a9 9 0 0 0-9-9Z"
                    />
                  </svg>
                </button>
              </div>
              <span
                v-if="usageLabel(item.sha256)"
                class="border-(--app-secondary-border) text-(--app-secondary-text) shrink-0 rounded border px-1.5 text-xs"
                >{{ usageLabel(item.sha256) }}</span
              >
              <button
                class="ml-auto shrink-0 cursor-pointer transition duration-300 hover:-translate-y-0.5 hover:brightness-200"
                :class="
                  usage[item.sha256]?.pinned
                    ? 'text-(--app-primary-text)'
                    : 'text-(--app-secondary-text)'
                "
                :title="usage[item.sha256]?.pinned ? 'Unpin' : 'Pin to top'"
                @click="imageStore.togglePinned(item.sha256)"
              >
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  viewBox="0 0 24 24"
                  stroke-width="1.0"
                  stroke="currentColor"
                  :fill="usage[item.sha256]?.pinned ? 'currentColor' : 'none'"
                  class="size-5"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    d="M17.593 3.322c1.1.128 1.907 1.077 1.907 2.185V21L12 17.25 4.5 21V5.507c0-1.108.806-2.057 1.907-2.185a48.507 48.507 0 0 1 11.186 0Z"
                  />
                </svg>
              </button>
              <!-- <ImageEditPopover
                @delete-event="() => imageStore.deleteImage(item.sha256)"
                class="ml-auto"
              /> -->
            </div>
            <div
              class="grid w-[256px] grid-cols-[24px_1fr_24px_1fr] items-center gap-1 pt-1 opacity-70"
            >
              <svg
                xmlns="http://www.w3.org/2000/svg"
                fill="none"
                viewBox="0 0 24 24"
                stroke-width="1.0"
                stroke="currentColor"
                class="text-(--app-secondary-text) size-5"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  d="M6.75 2.994v2.25m10.5-2.25v2.25m-14.252 13.5V7.491a2.25 2.25 0 0 1 2.25-2.25h13.5a2.25 2.25 0 0 1 2.25 2.25v11.251m-18 0a2.25 2.25 0 0 0 2.25 2.25h13.5a2.25 2.25 0 0 0 2.25-2.25m-18 0v-7.5a2.25 2.25 0 0 1 2.25-2.25h13.5a2.25 2.25 0 0 1 2.25 2.25v7.5m-6.75-6h2.25m-9 2.25h4.5m.002-2.25h.005v.006H12v-.006Zm-.001 4.5h.006v.006h-.006v-.005Zm-2.25.001h.005v.006H9.75v-.006Zm-2.25 0h.005v.005h-.006v-.005Zm6.75-2.247h.005v.005h-.005v-.005Zm0 2.247h.006v.006h-.006v-.006Zm2.25-2.248h.006V15H16.5v-.005Z"
                /></svg
              ><span class="text-(--app-secondary-text) text-sm">{{
                formatCreated(item.metadata)
              }}</span
              ><svg
                xmlns="http://www.w3.org/2000/svg"
                fill="none"
                viewBox="0 0 24 24"
                stroke-width="1.5"
                stroke="currentColor"
                class="text-(--app-secondary-text) size-5"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  d="M12 9.75v6.75m0 0-3-3m3 3 3-3m-8.25 6a4.5 4.5 0 0 1-1.41-8.775 5.25 5.25 0 0 1 10.233-2.33 3 3 0 0 1 3.758 3.848A3.752 3.752 0 0 1 18 19.5H6.75Z"
                />
              </svg>
              <span class="text-(--app-secondary-text) text-sm">{{
                formatSize(item.metadata)
              }}</span>
            </div>
          </div>
        </div>
      </div>
      <p v-else-if="isLoading" class="text-(--app-secondary-text) mt-6">Loading images...</p>
      <p v-else-if="error" class="text-(--app-danger) mt-6">
        Could not load images.
        <button class="cursor-pointer underline" @click="imageStore.refreshImages()">Retry</button>
      </p>
      <p v-else class="text-(--app-secondary-text) mt-6">
        {{ searchValue ? 'No images match your search.' : 'No images uploaded yet.' }}
      </p>
    </FadeInOut>
  </div>
</template>
