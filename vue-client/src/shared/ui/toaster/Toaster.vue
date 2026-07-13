<!--
SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>

SPDX-License-Identifier: Apache-2.0
-->

<script setup lang="ts">
import FadeInOut from '@/shared/ui/transitions/FadeInOut.vue'
import { useToasterStore } from '@/core/stores/toaster.store'
import { toTitleCase } from '@/shared/lib/pipes/titlecase'
import type { AlertSeverity } from '@/shared/types/toaster.model'
import { storeToRefs } from 'pinia'

const _toasterStore = useToasterStore()

const toasterRefs = storeToRefs(_toasterStore)

const icons: Record<AlertSeverity, string> = {
  success: `
  <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="size-8">
    <path stroke-linecap="round" stroke-linejoin="round" d="m11.25 11.25.041-.02a.75.75 0 0 1 1.063.852l-.708 2.836a.75.75 0 0 0 1.063.853l.041-.021M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Zm-9-3.75h.008v.008H12V8.25Z" />
  </svg>
  `,
  warning: `
  <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="size-8">
    <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m9-.75a9 9 0 1 1-18 0 9 9 0 0 1 18 0Zm-9 3.75h.008v.008H12v-.008Z" />
  </svg>`,
  error: `
  <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="size-8">
    <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126ZM12 15.75h.007v.008H12v-.008Z" />
  </svg>
  `,
}

const iconColor: Record<AlertSeverity, string> = {
  success: 'text-(--app-success)',
  warning: 'text-(--app-warning)',
  error: 'text-(--app-danger)',
}

const boxShadow: Record<AlertSeverity, string> = {
  success: 'shadow-[0_8px_30px_var(--app-success)]/30',
  warning: 'shadow-[0_8px_30px_var(--app-warning)]/30',
  error: 'shadow-[0_8px_30px_var(--app-danger)]/30',
}
</script>
<template>
  <FadeInOut>
    <div
      v-if="toasterRefs.activeAlert.value"
      v-bind:class="boxShadow[toasterRefs.activeAlert.value.alertSeverity]"
      class="border-(--app-secondary-border) bg-(--app-bg)/70 fixed right-3 top-3 z-[9997] flex max-h-[240px] min-h-[160px] w-[320px] flex-col gap-1 rounded-lg border-[1px] p-4 backdrop-blur-md md:right-12 md:top-12"
    >
      <div class="flex w-full items-center justify-between">
        <h6 class="text-xl">{{ toTitleCase(toasterRefs.activeAlert.value.title) }}</h6>
        <div
          class="size-8"
          v-bind:class="iconColor[toasterRefs.activeAlert.value.alertSeverity]"
          v-html="icons[toasterRefs.activeAlert.value.alertSeverity]"
        ></div>
      </div>
      <span class="text-(--app-secondary-text) font-light">{{
        toasterRefs.activeAlert.value.message
      }}</span>
    </div>
  </FadeInOut>
</template>
