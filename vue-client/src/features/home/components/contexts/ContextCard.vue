<!--
SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>

SPDX-License-Identifier: Apache-2.0
-->
<!---->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<script setup lang="ts">
import { useContextStore } from '@/core/stores/context-store'
import type { LocalMachine } from '@/shared/types/contexts.model'
import Tooltip from '@/shared/ui/tooltip/Tooltip.vue'
import { computed, toRef, watch } from 'vue'
import { useRouter } from 'vue-router'

const contextsStore = useContextStore()

const router = useRouter()

const props = defineProps<{
  machine: LocalMachine
}>()

const machine = toRef(props, 'machine')

const emits = defineEmits<{
  (e: 'onOpenSerial', name: string): void
  (e: 'onOpenVNC', name: string): void
}>()

const getMachineColor = (item: LocalMachine) => {
  if (item.powerState === 'on') {
    return 'text-(--app-success)'
  } else if (item.powerState === 'off') {
    return 'text-(--app-danger)'
  } else {
    return 'text-(--app-secondary-text)'
  }
}

function openSerial(e: MouseEvent) {
  e.preventDefault()
  if (machine.value.powerState !== 'on') return
  emits('onOpenSerial', props.machine.name)
}

function openVNV(e: MouseEvent) {
  e.preventDefault()
  if (machine.value.powerState !== 'on') return
  emits('onOpenVNC', props.machine.name)
}

async function onTogglePower(e: MouseEvent) {
  e.stopImmediatePropagation()
  await contextsStore.toggleMachinePower(machine.value.name)
}

const canToggle = computed(() => machine.value.powerState === 'on' || machine.value.activeImage)
</script>

<template>
  <div class="app-card relative flex h-[256px] w-[384px] flex-col">
    <div class="flex items-center justify-between">
      <div class="flex w-full flex-col">
        <div class="flex w-full items-center justify-between gap-2">
          <h6 class="text-xl">
            {{ machine.name }}
          </h6>
          <Tooltip
            :options="{
              message: canToggle ? 'Toggle power' : 'Provide an image to toggle power',
            }"
          >
            <button
              :disabled="!canToggle"
              :class="!canToggle && 'hover:-translate-0! opacity-30'"
              class="mb-auto duration-300 hover:-translate-y-0.5 hover:brightness-125"
              @click="onTogglePower"
            >
              <svg
                xmlns="http://www.w3.org/2000/svg"
                fill="none"
                viewBox="0 0 24 24"
                stroke-width="1.5"
                stroke="currentColor"
                class="size-6"
                :class="getMachineColor(machine)"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  d="M5.636 5.636a9 9 0 1 0 12.728 0M12 3v9"
                />
              </svg>
            </button>
          </Tooltip>
        </div>

        <span class="text-(--app-secondary-text) text-sm">{{ machine.machine_id }}</span>
      </div>
    </div>

    <div class="mt-auto flex items-center justify-between">
      <span class="text-(--app-secondary-text) text-sm">{{ machine.platform }}</span>
      <div class="bg-(--app-bg) flex items-center gap-4 rounded-full">
        <Tooltip
          :options="{
            message: machine.powerState === 'on' ? 'Connect To VNC' : 'Must Boot Machine First',
            yOffsetOverride: 32,
          }"
        >
          <button
            :disabled="machine.powerState !== 'on'"
            class="transition-opacity disabled:opacity-40"
            @click="openVNV"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 24 24"
              stroke-width="1.5"
              stroke="currentColor"
              class="text-(--app-secondary-text)/40 size-6"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                d="M9 17.25v1.007a3 3 0 0 1-.879 2.122L7.5 21h9l-.621-.621A3 3 0 0 1 15 18.257V17.25m6-12V15a2.25 2.25 0 0 1-2.25 2.25H5.25A2.25 2.25 0 0 1 3 15V5.25m18 0A2.25 2.25 0 0 0 18.75 3H5.25A2.25 2.25 0 0 0 3 5.25m18 0V12a2.25 2.25 0 0 1-2.25 2.25H5.25A2.25 2.25 0 0 1 3 12V5.25"
              />
            </svg>
          </button>
        </Tooltip>
        <Tooltip
          :options="{
            message: machine.powerState === 'on' ? 'Connect To Serial' : 'Must Boot Machine First',
            yOffsetOverride: 32,
          }"
        >
          <button
            class="transition-opacity disabled:opacity-40"
            :disabled="machine.powerState !== 'on'"
            @click="openSerial"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 24 24"
              stroke-width="1.5"
              stroke="currentColor"
              class="text-(--app-secondary-text)/40 size-6"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                d="m6.75 7.5 3 2.25-3 2.25m4.5 0h3m-9 8.25h13.5A2.25 2.25 0 0 0 21 18V6a2.25 2.25 0 0 0-2.25-2.25H5.25A2.25 2.25 0 0 0 3 6v12a2.25 2.25 0 0 0 2.25 2.25Z"
              />
            </svg>
          </button>
        </Tooltip>
        <Tooltip :options="{ message: 'Machine settings', yOffsetOverride: 32 }">
          <button
            @click="
              () =>
                router.push({
                  name: 'machineEdit',
                  params: { machineId: machine.machine_id },
                })
            "
            class="duration-300 hover:-translate-y-0.5 hover:brightness-200"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 24 24"
              stroke-width="1.5"
              stroke="currentColor"
              class="text-(--app-secondary-text)/40 size-6"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                d="M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.325.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 0 1 1.37.49l1.296 2.247a1.125 1.125 0 0 1-.26 1.431l-1.003.827c-.293.241-.438.613-.43.992a7.723 7.723 0 0 1 0 .255c-.008.378.137.75.43.991l1.004.827c.424.35.534.955.26 1.43l-1.298 2.247a1.125 1.125 0 0 1-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.47 6.47 0 0 1-.22.128c-.331.183-.581.495-.644.869l-.213 1.281c-.09.543-.56.94-1.11.94h-2.594c-.55 0-1.019-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 0 1-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 0 1-1.369-.49l-1.297-2.247a1.125 1.125 0 0 1 .26-1.431l1.004-.827c.292-.24.437-.613.43-.991a6.932 6.932 0 0 1 0-.255c.007-.38-.138-.751-.43-.992l-1.004-.827a1.125 1.125 0 0 1-.26-1.43l1.297-2.247a1.125 1.125 0 0 1 1.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.086.22-.128.332-.183.582-.495.644-.869l.214-1.28Z"
              />
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                d="M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z"
              />
            </svg>
          </button>
        </Tooltip>
      </div>
    </div>
  </div>
</template>
