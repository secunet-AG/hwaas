<script setup lang="ts">
import { useSerialWebsocket } from '@/shared/lib/hooks/useSerialWebsocket'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import { AttachAddon } from '@xterm/addon-attach'
import { ref, watch, onUnmounted, onMounted } from 'vue'
import { templateRef, useMemoize, useWindowSize } from '@vueuse/core'
import { useTerminalStore } from '../hooks/useTerminalStore'
import { SerializeAddon } from '@xterm/addon-serialize'

const { getHistoryForTerminal, setHistory } = useTerminalStore()

const props = defineProps<{
  isOpen: boolean
  machineName: string
  contextId: string
  port: string
}>()

const emits = defineEmits<{
  (e: 'onClose'): void
}>()

const width = useWindowSize()

watch(width, () => {
  fitAddonRef.value?.fit()
})

const { wss, setActiveMachineAndPort, unsubscribe } = useSerialWebsocket()

const terminalContainer = templateRef('terminalContainer')

const term = ref<Terminal>()
const fitAddonRef = ref<FitAddon>()

const getCSSVariables = useMemoize(() => {
  const style = window.getComputedStyle(document.body)
  return {
    background: style.getPropertyValue('--app-bg-transparent'),
    cursor: style.getPropertyValue('--app-primary-text'),
    white: style.getPropertyValue('--app-primary-text'),
    green: style.getPropertyValue('--app-success'),
    brightGreen: style.getPropertyValue('--app-success'),
    red: style.getPropertyValue('--app-danger'),
    magenta: style.getPropertyValue('--app-danger'),
    yellow: style.getPropertyValue('--app-warning'),
    brightYellow: style.getPropertyValue('--app-warning'),
    blue: style.getPropertyValue('--app-blue'),
    brightBlue: style.getPropertyValue('--app-blue'),
  }
})

function buildTerminal() {
  if (!terminalContainer.value || !wss.value) return
  const newTerm = new Terminal({
    fontFamily: 'Space Mono, monospace',
    cursorWidth: 3,
    cursorBlink: true,
    cols: 80,
    rows: 24,
    convertEol: true,
    fontSize: 16,
    allowTransparency: true,

    theme: getCSSVariables(),
  })

  const fitAddon = new FitAddon()
  newTerm.loadAddon(fitAddon)

  wss.value.addEventListener(
    'open',
    () => {
      if (!wss.value) return
      const attachAddon = new AttachAddon(wss.value)
      newTerm.loadAddon(attachAddon)
    },
    { once: true },
  )

  const serializeAddon = new SerializeAddon()
  newTerm.loadAddon(serializeAddon)

  newTerm.open(terminalContainer.value)

  fitAddon.fit()
  newTerm.focus()

  const existingData = getHistoryForTerminal(props.contextId, props.machineName)

  if (existingData) {
    newTerm.write(existingData)
  }

  newTerm.onLineFeed(() => {
    setHistory(props.contextId, props.machineName, serializeAddon.serialize())
  })

  term.value = newTerm
  fitAddonRef.value = fitAddon
}

watch(wss, () => buildTerminal(), { immediate: true })

onMounted(() => {
  setActiveMachineAndPort(props.machineName, props.port)
})

function closeTerminal() {
  unsubscribe()
  term.value?.dispose()
  emits('onClose')
}
</script>

<template>
  <div
    class="bg-(--app-bg)/5 fixed inset-0 z-[100] flex flex-col items-center justify-center p-12 backdrop-blur-[4px]"
  >
    <div
      class="app-card bg-(--app-bg)/60! rounded-2xl! p-6! py-3! w-full! grid h-full max-h-[1200px] max-w-[1200px] grid-rows-[auto_1fr_auto]"
    >
      <div class="flex w-full items-center justify-between pb-3">
        <button
          @click="
            () => {
              closeTerminal()
            }
          "
          class="z-30 ml-auto cursor-pointer"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            fill="none"
            viewBox="0 0 24 24"
            stroke-width="1.5"
            stroke="currentColor"
            class="text-(--app-secondary-text) size-8"
          >
            <path stroke-linecap="round" stroke-linejoin="round" d="M5 12h14" />
          </svg>
        </button>
      </div>

      <div ref="terminalContainer" class="h-full w-full shrink-0 overflow-y-auto"></div>
      <h6 class="text-(--app-secondary-text)/60! ml-auto pt-1 font-mono font-light">
        {{ props.machineName }}
      </h6>
    </div>
  </div>
</template>

<style>
@import '@xterm/xterm/css/xterm.css';
</style>
