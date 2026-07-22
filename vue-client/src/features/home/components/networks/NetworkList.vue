<script setup lang="ts">
import { storeToRefs } from 'pinia'
import NetworkCard from './NetworkCard.vue'
import { useContextStore } from '@/core/stores/context-store'
import { computed, watch } from 'vue'
import { type LocalNetwork, type Network } from '@/shared/types/contexts.model'

const contextsStore = useContextStore()
const contextRef = storeToRefs(contextsStore)

const networks = computed<LocalNetwork[]>(() => {
  // Convert the KV shape of machines to an array
  return (
    contextRef.activeContext.value?.networks?.map((x: Network) => ({
      name: x.name,
      networkMachines: Object.entries(x.machines ?? {}).map(([k, v]) => ({
        name: k,
        port: Object.keys(v)[0],
      })),
    })) ?? []
  )
})
</script>

<template>
  <div v-if="networks.length" class="flex flex-col gap-3 pt-6">
    <div v-for="item of networks" class="flex w-full items-center justify-between">
      <NetworkCard :network="item" />
    </div>
  </div>
</template>
