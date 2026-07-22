<!--
SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>

SPDX-License-Identifier: Apache-2.0
-->

<script setup lang="ts">
import { useContextStore } from '@/core/stores/context-store'
import { parseLifetime, secondsToReadable } from '@/shared/lib/validators/time-validation'
import { Dialog, DialogPanel, TransitionChild, TransitionRoot } from '@headlessui/vue'
import { templateRef } from '@vueuse/core'
import { storeToRefs } from 'pinia'
import { computed, nextTick, ref } from 'vue'

const MAX_LIFETIME_IN_SECONDS = import.meta.env.VITE_MAXIMUM_CONTEXT_LIFETIME_IN_SECONDS

const MAX_LIFETIME_IN_HOURS = Math.floor(MAX_LIFETIME_IN_SECONDS / 3600)

const displayMessage = computed(() => {
  let lifetime = _contextsStore.activeContext.value?.lifetime ?? 0
  return secondsToReadable(lifetime)
})

const contextsStore = useContextStore()

const inputRef = templateRef('inputRef')

const _contextsStore = storeToRefs(contextsStore)

const isOpen = ref(false)

const secondsToAdd = ref(0)

const didError = ref(false)

async function openModal() {
  isOpen.value = true
  await nextTick()
  inputRef.value?.focus()
}

function closeModal() {
  isOpen.value = false
  didError.value = false
}

function onLifetimeChange(e: any) {
  try {
    secondsToAdd.value = parseLifetime(e.target.value)
  } catch {
    secondsToAdd.value = 0
  }
}

async function onTimeAdd() {
  const activeContextId = contextsStore.activeContext?.id
  if (!activeContextId) throw new Error('No active context')

  const handleError = () => {
    secondsToAdd.value = 0
    didError.value = true
  }

  try {
    const res = await contextsStore.setLifetime(activeContextId, secondsToAdd.value)
    if (res >= 400) {
      handleError()
      return
    }
    closeModal()
  } catch (err) {
    console.error(err)
    handleError()
  }
}

const clockColor = computed(() => {
  const lifetime = contextsStore.activeContext?.lifetime
  if (!lifetime) {
    return {
      icon: 'text-(--app-danger)/60',
      border: 'text-(--app-danger)/60',
    }
  }

  return {
    icon: lifetime >= 60 * 10 ? 'text-(--app-primary-text)' : 'text-(--app-danger)',
    border: lifetime >= 60 * 10 ? 'border-(--app-primary-text)' : 'border-(--app-danger)',
  }
})
</script>

<template>
  <div class="flex items-center justify-center">
    <button
      @click="openModal"
      class="flex h-12 w-12 items-center justify-center rounded-full border-[1px]"
      :class="clockColor.border"
    >
      <svg
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 24 24"
        fill="currentColor"
        class="size-8"
        :class="clockColor.icon"
      >
        <path
          fill-rule="evenodd"
          d="M12 2.25c-5.385 0-9.75 4.365-9.75 9.75s4.365 9.75 9.75 9.75 9.75-4.365 9.75-9.75S17.385 2.25 12 2.25ZM12.75 6a.75.75 0 0 0-1.5 0v6c0 .414.336.75.75.75h4.5a.75.75 0 0 0 0-1.5h-3.75V6Z"
          clip-rule="evenodd"
        />
      </svg>
    </button>
  </div>

  <TransitionRoot appear :show="isOpen" as="template">
    <Dialog as="div" class="relative" @close="closeModal">
      <!-- Backdrop -->
      <div class="bg-(--app-bg)/40 fixed inset-0 backdrop-blur-lg transition-opacity" />

      <div class="fixed inset-0 overflow-y-auto">
        <div class="flex min-h-full items-center justify-center p-4 text-center">
          <TransitionChild
            as="template"
            enter="ease-out duration-300"
            enter-from="opacity-0 scale-95"
            enter-to="opacity-100 scale-100"
            leave="ease-in duration-200"
            leave-from="opacity-100 scale-100"
            leave-to="opacity-0 scale-95"
          >
            <DialogPanel
              class="border-(--app-secondary-border) bg-(--app-bg) w-full max-w-md transform overflow-hidden rounded-2xl border-2 p-6 text-left align-middle shadow-xl transition-all"
            >
              <div class="flex flex-col gap-3">
                <h2 class="text-[16px]">Context Lifetime</h2>
                <p class="text-(--app-secondary-text)!">
                  The maximum cummulative context lifetime is
                  {{ MAX_LIFETIME_IN_HOURS }} {{ MAX_LIFETIME_IN_HOURS === 1 ? 'hour' : 'hours' }}.
                  <br />
                  You have {{ displayMessage }}
                </p>
                <label>Update Lifetime</label>
                <div
                  class="border-(--app-secondary-border) grid w-full grid-cols-[1fr_auto] gap-3 border-b-[1px] pb-6"
                >
                  <div class="flex flex-col">
                    <input
                      ref="inputRef"
                      @keyup="(e: any) => onLifetimeChange(e)"
                      class="app-input min-w-0! appearance-none"
                      type="text"
                      placeholder="e.g 1h 30m 15s"
                    />
                    <span class="text-(--app-danger) mt-1 text-sm" v-if="didError">
                      Did you exceed the max cumulative lifetime?
                    </span>
                  </div>
                </div>

                <div class="mt-4 flex w-full flex-col gap-6">
                  <button @click="closeModal" type="button" class="app-btn-secondary w-full">
                    Cancel
                  </button>
                  <button
                    :disabled="!secondsToAdd"
                    @click="onTimeAdd"
                    class="app-btn-primary w-full"
                  >
                    Set Time
                  </button>
                </div>
              </div>
            </DialogPanel>
          </TransitionChild>
        </div>
      </div>
    </Dialog>
  </TransitionRoot>
</template>
