<!--
SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>

SPDX-License-Identifier: Apache-2.0
-->

<script setup lang="ts">
import { ref } from 'vue'
import NetworkCardPopover from './NetworkCardPopover.vue'
import Tooltip from '@/shared/ui/tooltip/Tooltip.vue'
import { useNetworksApi } from '@/shared/lib/api/networks/networks.api'
import { useContextStore } from '@/core/stores/context-store'
import NetworkUpdateModal from './NetworkUpdateModal.vue'
import type { LocalNetwork } from '@/shared/types/contexts.model'
import { useSpinnerStore } from '@/core/stores/spinner.store'

const isOpen = ref(false)

const contextsStore = useContextStore()

const { deleteNetwork, deleteMachineFromNetworkByName } = useNetworksApi()

const { setIsLoading } = useSpinnerStore()

const toggleIsOpen = () => {
  isOpen.value = !isOpen.value
}

const props = defineProps<{
  network: LocalNetwork
}>()

async function onDeleteNetwork() {
  const activeId = contextsStore.activeContext?.id
  if (!activeId) throw new Error('No active ID for finding network')
  setIsLoading(true)
  await deleteNetwork(activeId, props.network.name)
  await contextsStore.invalidateContextById(activeId)
  setIsLoading(false)
}

async function onDeleteMachineFromNetwork(payload: { name: string; port: string }) {
  const activeId = contextsStore.activeContext?.id
  if (!activeId) throw new Error('No active ID for finding network')
  setIsLoading(true)
  await deleteMachineFromNetworkByName(activeId, props.network.name, payload)
  await contextsStore.invalidateContextById(activeId)
  setIsLoading(false)
}
</script>

<template>
  <div class="app-card h-auto w-full">
    <div class="flex w-full items-center justify-between">
      <div class="flex flex-col">
        <h6>{{ props.network.name }}</h6>
      </div>
      <button @click="toggleIsOpen">
        <svg
          xmlns="http://www.w3.org/2000/svg"
          fill="none"
          viewBox="0 0 24 24"
          stroke-width="1.5"
          stroke="currentColor"
          class="text-(--app-secondary-text) size-6"
          :class="isOpen && 'rotate-180'"
        >
          <path stroke-linecap="round" stroke-linejoin="round" d="m4.5 15.75 7.5-7.5 7.5 7.5" />
        </svg>
      </button>
    </div>

    <div
      class="border-(--app-secondary-border) mt-3 flex w-full flex-col rounded-md border-[1px]"
      v-show="isOpen"
    >
      <div
        class="border-(--app-secondary-border) flex items-center justify-between gap-3 border-b-[1px] p-3"
      >
        <h6>Edit Network</h6>
        <div class="flex items-center gap-3">
          <NetworkUpdateModal :network="props.network"></NetworkUpdateModal>
          <button
            @click="onDeleteNetwork"
            class="app-btn-secondary-small text-(--app-danger)/90! min-w-[96px]"
          >
            Delete
          </button>
        </div>
      </div>

      <div
        class="border-(--app-secondary-border) grid max-h-[256px] w-auto grid-cols-[repeat(2,auto)] gap-3 overflow-visible border-b-[1px] p-3 last:border-b-0 md:grid-cols-[repeat(3,auto)] xl:grid-cols-[repeat(5,auto)]"
      >
        <div
          v-for="item of props.network.networkMachines"
          class="app-flag-dark flex max-w-[340px] shrink-0"
        >
          <div class="flex items-center gap-3">
            <h6 class="max-w-[128px] overflow-hidden text-ellipsis text-nowrap text-sm">
              {{ item.name }}
            </h6>
            <div class="text-(--app-secondary-text)/40 ml-auto">|</div>
            <Tooltip :options="{ message: 'Machine Network Port' }">
              <div
                class="border-(--app-secondary-border) flex min-w-[48px] cursor-pointer items-center justify-center rounded-full border-[1px] p-1"
              >
                <h6 class="text-(--app-secondary-text)/40! select-none px-3 text-sm">
                  {{ item.port }}
                </h6>
              </div>
            </Tooltip>
          </div>

          <div class="app-shadow absolute left-1 top-1 size-2 rounded-full"></div>
          <NetworkCardPopover @delete="onDeleteMachineFromNetwork(item)" class="ml-auto" />
        </div>
      </div>
    </div>
  </div>
</template>
