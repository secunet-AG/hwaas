<!--
SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>

SPDX-License-Identifier: Apache-2.0
-->

<script setup lang="ts">
import {
  Combobox,
  ComboboxButton,
  ComboboxInput,
  ComboboxOptions,
  ComboboxOption,
} from '@headlessui/vue'
import { useImagesStore } from '@/core/stores/images-store'
import { computed, ref } from 'vue'
import type { ImageMeta, LocalImage } from '@/shared/types/images.model'
import FadeInOut from '@/shared/ui/transitions/FadeInOut.vue'
import type { LocalMachine } from '@/shared/types/contexts.model'

const props = defineProps<{
  activeMachine: LocalMachine
}>()

const imageStore = useImagesStore()

// Images without metadata are skipped as they lack the fields LocalImage requires.
const imagesWithMeta = computed(() =>
  imageStore.sortedImages.flatMap(({ metadata }) => (metadata ? [metadata] : [])),
)

const architectures = computed(() =>
  [
    ...new Set(imagesWithMeta.value.flatMap((x) => (x.architecture ? [x.architecture] : []))),
  ].sort(),
)

const query = ref('')
const architectureFilter = ref<string | null>(null)

function matchesQuery(meta: ImageMeta, q: string) {
  return (
    meta.sha256.includes(q) ||
    meta.file_name.toLowerCase().includes(q) ||
    (meta.tags ?? []).some((t) => t.toLowerCase().includes(q))
  )
}

// Machines store images as LocalImage, so map from the store's ImageMeta.
const filteredImages = computed<LocalImage[]>(() => {
  const q = query.value.trim().toLowerCase()
  return imagesWithMeta.value
    .filter((x) => !architectureFilter.value || x.architecture === architectureFilter.value)
    .filter((x) => !q || matchesQuery(x, q))
    .map((x) => ({
      image_hash: x.sha256,
      file_name: x.file_name,
      size: x.size,
      created: x.created,
    }))
})

function usageLabel(hash: string) {
  const u = imageStore.usage[hash]
  if (u?.pinned) return 'Pinned'
  if (u?.lastUsedAt || u?.uploadedAt) return 'Recent'
  return null
}

function toggleArchitecture(arch: string) {
  architectureFilter.value = architectureFilter.value === arch ? null : arch
}

const selectedImage = ref<LocalImage | null>(props.activeMachine.activeImage ?? null)

function onSelection(image: LocalImage | null) {
  query.value = ''
  if (image) emit('onImageSelection', image)
}

const emit = defineEmits<{
  (event: 'onImageSelection', payload: LocalImage): void
}>()
</script>

<template>
  <div
    class="border-(--app-secondary-border) relative flex flex-col gap-2 rounded-md border-[1px] p-3"
  >
    <div v-if="architectures.length > 1" class="flex flex-wrap gap-2">
      <button
        v-for="arch in architectures"
        :key="arch"
        class="border-(--app-secondary-border) cursor-pointer rounded border px-2 text-xs transition duration-300"
        :class="
          architectureFilter === arch
            ? 'text-(--app-primary-text) bg-(--app-secondary-border)'
            : 'text-(--app-secondary-text) hover:text-(--app-primary-text)'
        "
        @click="toggleArchitecture(arch)"
      >
        {{ arch }}
      </button>
    </div>

    <Combobox v-model="selectedImage" by="image_hash" @update:model-value="onSelection">
      <div class="flex w-full items-center justify-between gap-3">
        <ComboboxInput
          class="placeholder:text-(--app-secondary-text) text-(--app-primary-text) w-full min-w-0 outline-none"
          placeholder="Search images by name, hash or tag..."
          :display-value="(x) => (x as LocalImage | null)?.file_name ?? ''"
          @change="query = $event.target.value"
        />
        <ComboboxButton
          class="shrink-0 transition duration-300 hover:-translate-y-0.5 hover:brightness-200"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            fill="none"
            viewBox="0 0 24 24"
            stroke-width="1.0"
            stroke="currentColor"
            class="text-(--app-primary-text) h-6 w-6"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              d="M8.25 15 12 18.75 15.75 15m-7.5-6L12 5.25 15.75 9"
            />
          </svg>
        </ComboboxButton>
      </div>
      <span
        v-if="selectedImage"
        class="text-(--app-secondary-text) overflow-hidden text-ellipsis text-nowrap text-sm"
      >
        {{ selectedImage.image_hash }}
      </span>

      <FadeInOut>
        <ComboboxOptions
          class="bg-(--app-bg) border-(--app-secondary-border) app-primary-shadow absolute left-0 top-full z-10 mt-3 flex max-h-80 w-full flex-col overflow-y-auto rounded-md border-2"
        >
          <li
            v-if="!filteredImages.length"
            class="text-(--app-secondary-text) flex h-[42px] items-center pl-3 text-sm"
          >
            {{ imageStore.isLoading ? 'Loading images...' : 'No images match.' }}
          </li>
          <ComboboxOption
            v-for="image in filteredImages"
            :key="image.image_hash"
            :value="image"
            v-slot="{ active }"
            as="template"
          >
            <li
              class="border-(--app-secondary-border) flex h-[42px] cursor-pointer items-center overflow-hidden border-b-[1px] pl-3 transition-all duration-300"
              :class="active ? 'text-(--app-primary-text)' : 'text-(--app-secondary-text)'"
            >
              <div class="flex w-full grow-0 flex-nowrap items-center gap-3 text-nowrap pr-3">
                <span class="text-(--app-primary-text)"> {{ image.file_name }}</span
                ><span class="min-w-0 overflow-hidden text-ellipsis text-nowrap text-sm">
                  {{ image.image_hash }}
                </span>
                <span
                  v-if="usageLabel(image.image_hash)"
                  class="border-(--app-secondary-border) ml-auto shrink-0 rounded border px-1.5 text-xs"
                  >{{ usageLabel(image.image_hash) }}</span
                >
              </div>
            </li>
          </ComboboxOption>
        </ComboboxOptions>
      </FadeInOut>
    </Combobox>
  </div>
</template>
