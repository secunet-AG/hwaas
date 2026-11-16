<!--
SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>

SPDX-License-Identifier: Apache-2.0
-->

<script setup lang="ts">
import { computed, ref } from 'vue'
import { ARCHITECTURES, type Architecture } from '@/shared/types/images.model'

const props = defineProps<{
  knownTags: string[]
}>()

const architecture = defineModel<Architecture | null>('architecture', { required: true })
const tags = defineModel<string[]>('tags', { required: true })

const tagInput = ref('')

const suggestions = computed(() => {
  const q = tagInput.value.trim().toLowerCase()
  return props.knownTags
    .filter((x) => !tags.value.includes(x))
    .filter((x) => !q || x.toLowerCase().includes(q))
    .slice(0, 8)
})

function addTag(raw: string) {
  const tag = raw.trim()
  if (tag && !tags.value.includes(tag)) tags.value = [...tags.value, tag]
  tagInput.value = ''
}

function removeTag(tag: string) {
  tags.value = tags.value.filter((x) => x !== tag)
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' || e.key === ',') {
    e.preventDefault()
    addTag(tagInput.value)
  } else if (e.key === 'Backspace' && !tagInput.value && tags.value.length) {
    tags.value = tags.value.slice(0, -1)
  }
}

function toggleArchitecture(arch: Architecture) {
  architecture.value = architecture.value === arch ? null : arch
}
</script>

<template>
  <div class="flex flex-col gap-4">
    <div class="flex flex-col gap-2">
      <label class="text-sm">Architecture</label>
      <div class="grid grid-cols-3 gap-2">
        <button
          v-for="arch in ARCHITECTURES"
          :key="arch"
          type="button"
          class="border-(--app-secondary-border) cursor-pointer rounded border px-2 py-0.5 text-sm transition duration-300"
          :class="
            architecture === arch
              ? 'text-(--app-primary-text) bg-(--app-secondary-border)'
              : 'text-(--app-secondary-text) hover:text-(--app-primary-text)'
          "
          @click="toggleArchitecture(arch)"
        >
          {{ arch }}
        </button>
      </div>
    </div>

    <div class="flex flex-col gap-2">
      <label for="image-tag-input" class="text-sm">Tags</label>
      <div
        class="border-(--app-secondary-border) flex flex-wrap items-center gap-1 rounded-md border p-2"
      >
        <span
          v-for="tag in tags"
          :key="tag"
          class="bg-(--app-secondary-border) text-(--app-primary-text) flex items-center gap-1 rounded px-1.5 text-sm"
        >
          {{ tag }}
          <button
            type="button"
            class="text-(--app-secondary-text) hover:text-(--app-primary-text) cursor-pointer"
            :aria-label="`Remove tag ${tag}`"
            @click="removeTag(tag)"
          >
            ×
          </button>
        </span>
        <input
          id="image-tag-input"
          v-model="tagInput"
          placeholder="Add a tag and press Enter..."
          class="placeholder:text-(--app-secondary-text) text-(--app-primary-text) min-w-[128px] flex-1 text-sm outline-none"
          @keydown="onKeydown"
          @blur="addTag(tagInput)"
        />
      </div>
      <div v-if="suggestions.length" class="flex flex-wrap gap-1">
        <button
          v-for="tag in suggestions"
          :key="tag"
          type="button"
          class="border-(--app-secondary-border) text-(--app-secondary-text) hover:text-(--app-primary-text) cursor-pointer rounded border px-1.5 text-xs transition duration-300"
          @mousedown.prevent
          @click="addTag(tag)"
        >
          + {{ tag }}
        </button>
      </div>
    </div>
  </div>
</template>
