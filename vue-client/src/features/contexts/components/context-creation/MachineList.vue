<!--
SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>

SPDX-License-Identifier: Apache-2.0
-->

<script setup lang="ts">
import { computed, reactive, ref, onMounted } from 'vue'
import { nameGenerator } from '@/shared/lib/etc/machine-name-generator'
import type { InventoryMachine } from '@/shared/types/contexts.model'
import { useConfig } from '@/core/plugins/config-plugin'

const props = defineProps<{
  disabled: boolean
  options?: {
    hideSelected: boolean
  }
}>()

interface MachineInventoryStateItem extends InventoryMachine {
  selected: boolean
}

const emit = defineEmits<{
  (event: 'selectedChange', payload: InventoryMachine[]): void
}>()

const machineList = ref([] as MachineInventoryStateItem[])

const { API_URL } = useConfig()

async function fetchMachines(): Promise<MachineInventoryStateItem[]> {
  const res = await fetch(`${API_URL}/inventory`)
  const payload = (await res.json()) as InventoryMachine[]

  // Filter our free items and initialize with a non selected initial state
  return payload.reduce(
    (acc, cur) => (cur.state === 'free' ? [...acc, { ...cur, selected: false }] : [...acc]),
    [] as MachineInventoryStateItem[],
  )
}

const toggleAtIndex = (index: number) => {
  if (props.disabled) return

  machineList.value[index].selected = !machineList.value[index].selected
  emit(
    'selectedChange',
    machineList.value.filter((x) => x.selected).map((x) => ({ ...x, name: nameGenerator() })),
  )
}

onMounted(async () => {
  machineList.value = await fetchMachines()
})
</script>

<template>
  <div class="w-full" :class="props.disabled ? 'cursor-not-allowed opacity-40' : ''">
    <div
      class="border-(--app-primary-border) mt-3 flex h-[192px] w-full flex-col overflow-y-auto rounded-md border-[1px]"
    >
      <div v-for="(item, index) of machineList">
        <div
          v-if="!(props.options?.hideSelected && item.selected)"
          @click="() => toggleAtIndex(index)"
          :hey="item.machine_id"
          class="border-(--app-secondary-border) flex h-[48px] w-full shrink-0 items-center justify-between border-b-[1px] px-3"
        >
          <div class="grid grid-cols-[24px_1fr] items-center">
            <h6>{{ item.machine_id }}</h6>
            <span class="text-(--app-secondary-text)/80 overflow-ellipsis text-sm">{{
              item.properties.platform
            }}</span>
          </div>
          <div class="items-center gap-2">
            <svg
              v-if="item.selected"
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 24 24"
              stroke-width="1.0"
              stroke="currentColor"
              class="text-(--app-primary-text) size-6"
            >
              <path stroke-linecap="round" stroke-linejoin="round" d="m4.5 12.75 6 6 9-13.5" />
            </svg>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
