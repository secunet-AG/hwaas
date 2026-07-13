<!--
SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>

SPDX-License-Identifier: Apache-2.0
-->

<script setup lang="ts">
import { computed } from 'vue'
import { Listbox, ListboxButton, ListboxOptions, ListboxOption } from '@headlessui/vue'

import { useContextStore } from '@/core/stores/context-store'
import FadeInOut from '@/shared/ui/transitions/FadeInOut.vue'
import type { Context } from '@/shared/types/contexts.model'
import Tooltip from '@/shared/ui/tooltip/Tooltip.vue'

const contextStore = useContextStore()

const selectedContext = computed({
  get: () => contextStore.activeContext,
  set: (val: Context) => contextStore.setActiveContext(val.id),
})
function copyIdToClipboard(e: MouseEvent) {
  if (!selectedContext.value) return
  e.stopImmediatePropagation() // Prevent this from bubbling to search bar below
  navigator.clipboard.writeText(selectedContext.value.id)
}
</script>

<template>
  <div v-if="selectedContext" class="relative w-[368px]">
    <Listbox v-model="selectedContext">
      <ListboxButton class="flex w-full items-center justify-between">
        <div class="flex flex-col gap-1">
          <h6 class="text-left text-lg">{{ selectedContext.name }}</h6>
          <div class="flex gap-1">
            <span class="text-(--app-secondary-text) text-left text-sm">
              {{ selectedContext.id }}
            </span>
            <Tooltip :options="{ message: 'Copy Context ID to clipboard' }">
              <button
                class="flex cursor-pointer items-center gap-1 transition duration-300 hover:-translate-y-0.5 hover:brightness-200"
                v-on:click="copyIdToClipboard"
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
            </Tooltip>
          </div>
        </div>
        <button class="p-3 transition duration-300 hover:-translate-y-0.5 hover:brightness-200">
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
        </button>
      </ListboxButton>

      <FadeInOut>
        <ListboxOptions
          class="bg-(--app-bg) border-(--app-secondary-border) app-primary-shadow absolute left-0 top-14 mt-3 flex w-full flex-col rounded-md border-2"
        >
          <ListboxOption
            class="border-(--app-secondary-border) text-(--app-secondary-text) hover:text-(--app-primary-text) flex h-[42px] cursor-pointer items-center border-b-[1px] pl-3 transition-all duration-300"
            v-for="context in contextStore.contexts"
            :key="context.id"
            :value="context"
          >
            <span>
              {{ context.name }}
            </span>
          </ListboxOption>
        </ListboxOptions>
      </FadeInOut>
    </Listbox>
  </div>
</template>
