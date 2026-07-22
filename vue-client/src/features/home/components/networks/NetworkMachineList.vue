<script setup lang="ts">
import type { InventoryMachine, LocalMachine } from '@/shared/types/contexts.model'

const props = defineProps<{
  disabled: boolean
  machineList: LocalMachine[]
}>()

const emit = defineEmits<{
  (event: 'selectedChange', payload: LocalMachine): void
}>()

const toggleAtIndex = (item: LocalMachine) => {
  emit('selectedChange', item)
}
</script>

<template>
  <div class="w-full" :class="props.disabled ? 'cursor-not-allowed opacity-40' : ''">
    <div
      class="border-(--app-primary-border) mt-3 flex min-h-[128px] w-full flex-col overflow-y-auto rounded-md border-[1px]"
    >
      <div v-for="item of machineList">
        <div
          @click="() => toggleAtIndex(item)"
          :hey="item.machine_id"
          class="border-(--app-secondary-border) flex h-[48px] w-full shrink-0 items-center justify-between border-b-[1px] px-3"
        >
          <div class="flex items-center gap-3">
            <h6>{{ item.name }}</h6>
            <span class="text-(--app-secondary-text)">
              {{ item.machine_id }}
            </span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
