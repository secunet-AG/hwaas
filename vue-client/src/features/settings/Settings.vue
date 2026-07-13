<!--
SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>

SPDX-License-Identifier: Apache-2.0
-->

<script setup lang="ts">
import { useApiUrl } from '@/core/plugins/apiUrlPlugin'
import { useImagesStore } from '@/core/stores/images-store'
import Tooltip from '@/shared/ui/tooltip/Tooltip.vue'
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'

const api = useApiUrl()

const apiRef = ref(api.apiUrl.value)

const isDirty = computed(() => api.apiUrl.value !== apiRef.value)

const imageStore = useImagesStore()

const isValid = computed(() => {
  try {
    new URL(apiRef.value)
    return true
  } catch {
    return false
  }
})

const router = useRouter()
</script>

<template>
  <div class="flex h-full w-full flex-col gap-6">
    <div class="flex h-12 items-center">
      <h4 class="text-2xl">Settings</h4>
    </div>
    <div class="flex flex-col gap-3">
      <label class="text-(app-secondary-text)">API Url</label>
      <input v-model="apiRef" class="app-input" />
    </div>
    <div class="mt-auto flex w-full justify-center">
      <Tooltip
        class="w-full max-w-[1200px]"
        :options="{
          message: 'Please make sure that your API Url is valid',
          isDisabled: isValid,
          yOffsetOverride: -64,
        }"
      >
        <button
          @click="
            () => {
              api.setApiKey(apiRef)
              // When changing context APIs, we don't want to show stale images from prev. context
              imageStore.clearImageCache()
              router.push({ name: 'home' })
            }
          "
          v-if="isDirty"
          :disabled="!isValid"
          class="app-btn-primary w-full"
        >
          Save
        </button>
      </Tooltip>
    </div>
  </div>
</template>
