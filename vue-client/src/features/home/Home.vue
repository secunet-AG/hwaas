<!--
SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>

SPDX-License-Identifier: Apache-2.0
-->
<!---->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<script setup lang="ts">
import { useContextStore } from '@/core/stores/context-store'
import ContextHeader from '@/features/home/components/contexts/ContextHeader.vue'
import ContextCards from './components/contexts/ContextCardsContainer.vue'
import NetworkList from './components/networks/NetworkList.vue'
import { useRouter } from 'vue-router'
import NoContextsFound from '../contexts/components/context-creation/NoContextsFound.vue'
import { defineAsyncComponent, onMounted, ref } from 'vue'
import { storeToRefs } from 'pinia'
import NetworkCreationModal from './components/networks/NetworkCreationModal.vue'
import FadeInOut from '@/shared/ui/transitions/FadeInOut.vue'
import MJPEG from './components/machine-view/components/MJPEG.vue'
import type { LocalMachine } from '@/shared/types/contexts.model'

const TerminalAsync = defineAsyncComponent(
  () => import('./components/machine-view/components/Terminal.vue'),
)

const _contextsStore = useContextStore()
const contextStoreRef = storeToRefs(_contextsStore)

const router = useRouter()

const isSerialOpen = ref(false)
const isKVMOpen = ref(false)
const activeMachine = ref<LocalMachine | null>(null)

function setActiveMachine(machineName: string) {
  const machine = contextStoreRef.activeContext.value?.machines.find((x) => x.name === machineName)
  if (!machine) throw new Error('Could not find machine')
  activeMachine.value = machine
}

onMounted(async () => {
  const activeContextId = contextStoreRef.activeContext.value?.id
  if (!activeContextId) return
  await _contextsStore.invalidateContextById(activeContextId)
})

function onCloseTerminal() {
  isSerialOpen.value = false
  activeMachine.value = null
}

function onCloseKVM() {
  isKVMOpen.value = false
  activeMachine.value = null
}

function onOpenKVM(name: string) {
  setActiveMachine(name)
  isKVMOpen.value = true
}

function onOpenSerial(name: string) {
  setActiveMachine(name)
  isSerialOpen.value = true
}

function onEdit() {
  router.push({
    name: 'context-edit',
    params: {
      contextId: _contextsStore.activeContext?.id,
    },
  })
}
</script>

<template>
  <div class="flex h-full flex-col" v-if="contextStoreRef.contexts.value.length">
    <ContextHeader v-on:edit="onEdit" />

    <div class="flex flex-col pt-8">
      <h2 class="text-2xl">Machines</h2>
      <ContextCards @on-open-k-v-m="onOpenKVM" @on-open-serial="onOpenSerial" />
    </div>

    <FadeInOut>
      <MJPEG
        :context-id="contextStoreRef.activeContext.value?.id ?? ''"
        :machine-name="activeMachine.name"
        v-if="_contextsStore.activeContext?.id && activeMachine?.name && isKVMOpen"
        @on-close="onCloseKVM"
      ></MJPEG>
    </FadeInOut>

    <FadeInOut>
      <!-- TODO: Discuss how we want to configure serial ports -->
      <TerminalAsync
        v-if="activeMachine && isSerialOpen"
        @on-close="onCloseTerminal"
        :is-open="isSerialOpen"
        :context-id="contextStoreRef.activeContext.value?.id ?? ''"
        :machine-name="activeMachine.name"
        :port="activeMachine.serialPorts[0]"
      />
    </FadeInOut>

    <div class="mt-6 flex flex-col pt-3">
      <div class="flex items-center justify-between">
        <h2 class="text-2xl">Networks</h2>
        <NetworkCreationModal />
      </div>
      <NetworkList />
    </div>
  </div>
  <NoContextsFound v-else></NoContextsFound>
</template>
