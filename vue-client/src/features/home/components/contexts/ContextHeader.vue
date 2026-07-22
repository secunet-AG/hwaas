<!--
SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>

SPDX-License-Identifier: Apache-2.0
-->

<script setup lang="ts">
import ContextPicker from '@/features/home/components/contexts/ContextPicker.vue'
import ContextLifetimeModal from './ContextLifetimeModal.vue'
import Tooltip from '@/shared/ui/tooltip/Tooltip.vue'
import { storeToRefs } from 'pinia'
import { useContextStore } from '@/core/stores/context-store'
import { computed } from 'vue'
import { secondsToReadable } from '@/shared/lib/validators/time-validation'

const emit = defineEmits<{
  (event: 'edit'): void
}>()

const docsUrl = import.meta.env.VITE_DOCS_URL

const _contextsStore = useContextStore()
const contextsStore = storeToRefs(_contextsStore)

const currentLifetimeDisplay = computed(() =>
  secondsToReadable(_contextsStore.activeContext?.lifetime ?? 0),
)
</script>

<template>
  <div class="border-(--app-secondary-text) flex items-center justify-between border-b-[1px] pb-6">
    <ContextPicker />
    <div class="flex gap-6">
      <Tooltip :options="{ message: `${currentLifetimeDisplay}` }">
        <ContextLifetimeModal />
      </Tooltip>
      <Tooltip :options="{ message: 'Context Settings' }">
        <button
          @click="() => emit('edit')"
          class="border-(--app-primary-text) flex h-12 w-12 items-center justify-center rounded-full border-[1px]"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 24 24"
            fill="currentColor"
            class="text-(--app-primary-text) size-8"
          >
            <path
              fill-rule="evenodd"
              d="M12 6.75a5.25 5.25 0 0 1 6.775-5.025.75.75 0 0 1 .313 1.248l-3.32 3.319c.063.475.276.934.641 1.299.365.365.824.578 1.3.64l3.318-3.319a.75.75 0 0 1 1.248.313 5.25 5.25 0 0 1-5.472 6.756c-1.018-.086-1.87.1-2.309.634L7.344 21.3A3.298 3.298 0 1 1 2.7 16.657l8.684-7.151c.533-.44.72-1.291.634-2.309A5.342 5.342 0 0 1 12 6.75ZM4.117 19.125a.75.75 0 0 1 .75-.75h.008a.75.75 0 0 1 .75.75v.008a.75.75 0 0 1-.75.75h-.008a.75.75 0 0 1-.75-.75v-.008Z"
              clip-rule="evenodd"
            />
            <path
              d="m10.076 8.64-2.201-2.2V4.874a.75.75 0 0 0-.364-.643l-3.75-2.25a.75.75 0 0 0-.916.113l-.75.75a.75.75 0 0 0-.113.916l2.25 3.75a.75.75 0 0 0 .643.364h1.564l2.062 2.062 1.575-1.297Z"
            />
            <path
              fill-rule="evenodd"
              d="m12.556 17.329 4.183 4.182a3.375 3.375 0 0 0 4.773-4.773l-3.306-3.305a6.803 6.803 0 0 1-1.53.043c-.394-.034-.682-.006-.867.042a.589.589 0 0 0-.167.063l-3.086 3.748Zm3.414-1.36a.75.75 0 0 1 1.06 0l1.875 1.876a.75.75 0 1 1-1.06 1.06L15.97 17.03a.75.75 0 0 1 0-1.06Z"
              clip-rule="evenodd"
            />
          </svg>
        </button>
      </Tooltip>

      <Tooltip :options="{ message: 'Documentation' }">
        <a
          target="_blank"
          :href="docsUrl"
          class="border-(--app-primary-text) flex h-12 w-12 items-center justify-center rounded-full border-[1px]"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 24 24"
            fill="currentColor"
            class="text-(--app-primary-text) size-8"
          >
            <path
              fill-rule="evenodd"
              d="M2.25 12c0-5.385 4.365-9.75 9.75-9.75s9.75 4.365 9.75 9.75-4.365 9.75-9.75 9.75S2.25 17.385 2.25 12Zm11.378-3.917c-.89-.777-2.366-.777-3.255 0a.75.75 0 0 1-.988-1.129c1.454-1.272 3.776-1.272 5.23 0 1.513 1.324 1.513 3.518 0 4.842a3.75 3.75 0 0 1-.837.552c-.676.328-1.028.774-1.028 1.152v.75a.75.75 0 0 1-1.5 0v-.75c0-1.279 1.06-2.107 1.875-2.502.182-.088.351-.199.503-.331.83-.727.83-1.857 0-2.584ZM12 18a.75.75 0 1 0 0-1.5.75.75 0 0 0 0 1.5Z"
              clip-rule="evenodd"
            />
          </svg>
        </a>
      </Tooltip>
    </div>
  </div>
</template>
