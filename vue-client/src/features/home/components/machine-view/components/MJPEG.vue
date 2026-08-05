<!--
SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>

SPDX-License-Identifier: Apache-2.0
-->

<script setup lang="ts">
import { useConfig } from '@/core/plugins/config-plugin'

import { useContextStore } from '@/core/stores/context-store'
import { useKeyboardCapture, type KeyboardReport } from '@/shared/lib/hooks/useKeyboardCapture'
import { useKeyboardWebsocket } from '@/shared/lib/hooks/useKeyboardWebsocket'
import useMouseCapture from '@/shared/lib/hooks/useMouseCapture'
import KvmKeyboardToolbar, { type ComboEvent } from './KvmKeyboardToolbar.vue'
import { storeToRefs } from 'pinia'
import { computed, onBeforeUnmount, reactive, ref, useTemplateRef, watch } from 'vue'
import { Keyboard } from '@lucide/vue'

const props = defineProps<{
  machineName: string
  contextId: string
}>()

const isActive = ref(true)

const { sendMessage, setActiveMachineAndPort } = useKeyboardWebsocket()

watch(
  props,
  () => {
    setActiveMachineAndPort(props.machineName)
  },
  { immediate: true },
)

const onKeyboardPress = (r: KeyboardReport) => sendMessage(r)

// Modifiers the user has toggled in the toolbar.
//
// Shared with the keyboard capture so a sticky modifier applies to the next real or virtual keypress.
// This is then dropped after one application. UX decision, can be changed.
const stickyModifiers = reactive(new Set<string>())

useKeyboardCapture(isActive, onKeyboardPress, stickyModifiers)

const showKeyboardToolbar = ref(false)

// Key combinations are effectively a press, small wait, and release.
//
// Here we apply this on any combination of keys
function onToolbarSend(combo: ComboEvent) {
  sendMessage({ keys: combo.keys, modifier: combo.modifier, press: true, release: false })
  window.setTimeout(() => {
    sendMessage({ keys: [], modifier: 0, press: false, release: true })
  }, 100)
}

const { apiUrl } = useConfig()

const contextStore = useContextStore()
const contextStoreRefs = storeToRefs(contextStore)

const mjpegUrl = computed(() => {
  const activeContextId = contextStoreRefs.activeContext.value?.id

  return `${apiUrl.value}/contexts/${activeContextId}/machines/${props.machineName}/mjpeg`
})

onBeforeUnmount(() => {
  // Without this, Vue does not clean up the stream with the image
  if (imgRef.value) imgRef.value.src = ''
})

const isFullscreen = ref(false)

const imgRef = useTemplateRef('streamingImage')

const fullscreenContainerTemplateRef = useTemplateRef('screenContainer')

const currentWindowStyle = computed(() =>
  isFullscreen.value
    ? 'w-full h-full object-center'
    : 'app-card h-auto! w-auto! p-3 pt-0! bg-(--app-bg)/60! rounded-2xl!',
)

const toggleFullscreen = () => {
  isFullscreen.value = !isFullscreen.value
  if (isFullscreen.value) {
    fullscreenContainerTemplateRef.value?.requestFullscreen()
    return
  }
  document.exitFullscreen()
}

const emits = defineEmits(['onClose'])

useMouseCapture(imgRef, props.machineName)

function onClose() {
  emits('onClose')
}
</script>

<template>
  <div
    class="bg-(--app-bg)/5 fixed inset-0 z-[100] flex flex-col items-center justify-center backdrop-blur-[4px]"
  >
    <div ref="screenContainer" :class="currentWindowStyle" class="relative flex flex-col">
      <div class="flex w-full items-center justify-end gap-3 p-3">
        <button
          id="toggle-keyboard-toolbar"
          class="cursor-pointer"
          @click="showKeyboardToolbar = !showKeyboardToolbar"
        >
          <Keyboard
            class="w-8 text-(--app-primary-border) transition hover:text-(--app-primary-text)"
          ></Keyboard>
        </button>
        <button id="minimize" @click="onClose()" class="cursor-pointer">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            fill="none"
            viewBox="0 0 24 24"
            stroke-width="1.5"
            stroke="currentColor"
            class="w-8 text-(--app-primary-border) transition hover:text-(--app-primary-text)"
          >
            <path stroke-linecap="round" stroke-linejoin="round" d="M5 12h14" />
          </svg>
        </button>
        <button class="cursor-pointer" @click="toggleFullscreen" id="toggle-fullscreen">
          <svg
            v-if="isFullscreen"
            xmlns="http://www.w3.org/2000/svg"
            fill="none"
            viewBox="0 0 24 24"
            stroke-width="1.5"
            stroke="currentColor"
            class="w-8 text-(--app-primary-border) transition hover:text-(--app-primary-text)"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              d="M16.5 8.25V6a2.25 2.25 0 0 0-2.25-2.25H6A2.25 2.25 0 0 0 3.75 6v8.25A2.25 2.25 0 0 0 6 16.5h2.25m8.25-8.25H18a2.25 2.25 0 0 1 2.25 2.25V18A2.25 2.25 0 0 1 18 20.25h-7.5A2.25 2.25 0 0 1 8.25 18v-1.5m8.25-8.25h-6a2.25 2.25 0 0 0-2.25 2.25v6"
            />
          </svg>
          <svg
            v-else
            xmlns="http://www.w3.org/2000/svg"
            fill="none"
            viewBox="0 0 24 24"
            stroke-width="1.5"
            stroke="currentColor"
            class="w-8 text-(--app-primary-border) transition hover:text-(--app-primary-text)"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              d="M5.25 7.5A2.25 2.25 0 0 1 7.5 5.25h9a2.25 2.25 0 0 1 2.25 2.25v9a2.25 2.25 0 0 1-2.25 2.25h-9a2.25 2.25 0 0 1-2.25-2.25v-9Z"
            />
          </svg>
        </button>
      </div>
      <KvmKeyboardToolbar
        v-if="showKeyboardToolbar"
        :sticky-modifiers="stickyModifiers"
        @send="onToolbarSend"
      />
      <!-- The live mjpeg stream, for the price of an img tag!-->
      <img ref="streamingImage" class="w-full h-full object-fit rounded-lg" :src="mjpegUrl" />
    </div>
  </div>
</template>

<style>
#container > * {
  background: none !important;
}
</style>
