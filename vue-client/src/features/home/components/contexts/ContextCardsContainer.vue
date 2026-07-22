<script setup lang="ts">
import { useContextStore } from '@/core/stores/context-store'
import ContextCard from './ContextCard.vue'

const contextsStore = useContextStore()

const emits = defineEmits<{
  (e: 'onOpenSerial', name: string): void
  (e: 'onOpenVNC', name: string): void
}>()
</script>

<template>
  <div class="flex gap-6 pt-6">
    <ContextCard
      @on-open-serial="(name) => emits('onOpenSerial', name)"
      @on-open-v-n-c="(name) => emits('onOpenVNC', name)"
      :machine="item"
      v-for="item of contextsStore.activeMachines"
    ></ContextCard>
  </div>
</template>
