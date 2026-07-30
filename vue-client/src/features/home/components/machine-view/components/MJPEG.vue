<!--
SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>

SPDX-License-Identifier: Apache-2.0
-->

<script setup lang="ts">
import { useApiUrl } from '@/core/plugins/apiUrlPlugin'
import { useContextStore } from '@/core/stores/context-store'
import { useKeyboardCapture, type KeyboardReport } from '@/shared/lib/hooks/useKeyboardCapture'
import { useKeyboardWebsocket } from '@/shared/lib/hooks/useKeyboardWebsocket'
import useMouseCapture from '@/shared/lib/hooks/useMouseCapture'
import KvmKeyboardToolbar, { type ComboEvent } from './KvmKeyboardToolbar.vue'
import { storeToRefs } from 'pinia'
import { computed, onBeforeUnmount, ref, useTemplateRef, watch } from 'vue'

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

useKeyboardCapture(isActive, onKeyboardPress)

const showKeyboardToolbar = ref(false)

// Turn a key combo into a press followed by a release.
function onToolbarSend(combo: ComboEvent) {
  sendMessage({ keys: combo.keys, modifier: combo.modifier, press: true, release: false })
  window.setTimeout(() => {
    sendMessage({ keys: [], modifier: 0, press: false, release: true })
  }, 100)
}

const { apiUrl } = useApiUrl()

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
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="24"
            height="24"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
            stroke-linejoin="round"
            class="w-8 text-(--app-primary-border) transition hover:text-(--app-primary-text)"
          >
            <path d="M10 8h.01" />
            <path d="M12 12h.01" />
            <path d="M14 8h.01" />
            <path d="M16 12h.01" />
            <path d="M18 8h.01" />
            <path d="M6 8h.01" />
            <path d="M7 16h10" />
            <path d="M8 12h.01" />
            <rect width="20" height="16" x="2" y="4" rx="2" />
          </svg>
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
      <KvmKeyboardToolbar v-if="showKeyboardToolbar" @send="onToolbarSend" />
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
