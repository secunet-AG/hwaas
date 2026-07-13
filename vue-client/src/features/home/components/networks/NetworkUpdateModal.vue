<!--
SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>

SPDX-License-Identifier: Apache-2.0
-->

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { TransitionRoot, TransitionChild, Dialog, DialogPanel } from '@headlessui/vue'
import { useContextStore } from '@/core/stores/context-store'
import type { LocalMachine, LocalNetwork } from '@/shared/types/contexts.model'
import NetworkMachineList from './NetworkMachineList.vue'
import type { MachineWithPort } from '@/shared/types/contexts.model'
import { useModal } from '@/shared/lib/hooks/useModal'
import { useMachinesApi } from '@/shared/lib/api/machines/machines.api'
import { useNetworksApi } from '@/shared/lib/api/networks/networks.api'
import { useSpinnerStore } from '@/core/stores/spinner.store'

const props = defineProps<{
  network: LocalNetwork
}>()

const { isOpen, openModal, closeModal } = useModal()

const { getNetworkInterfaces } = useMachinesApi()

const { setIsLoading } = useSpinnerStore()

const { addMachinesToNetwork } = useNetworksApi()

const machines = ref<MachineWithPort[]>([])

const machineToEdit = ref<MachineWithPort | null>(null)

const contextStore = useContextStore()

async function fetchInterfaces(): Promise<MachineWithPort[]> {
  const machinesInNetwork = new Set(props.network.networkMachines.map((x) => x.name))

  let selectedMachines = contextStore.activeContext?.machines.filter(
    (x) => !machinesInNetwork.has(x.name),
  ) // We need to filter out machines currently selected

  if (!selectedMachines) throw new Error('No machines selected on context')

  selectedMachines = selectedMachines.map((x) => ({ ...x, isSelected: false }))

  if (!contextStore?.activeContext?.id) throw new Error('No active context')

  const awaitedMachines = await Promise.all(
    selectedMachines.map(async (x) => {
      const serialRes = await getNetworkInterfaces(contextStore!.activeContext!.id, String(x.name))
      return { ...x, isSelected: false, ports: serialRes.data ? serialRes.data : [] }
    }),
  )

  return awaitedMachines as any
}

const onSelectionWithPort = (port: string) => {
  if (!machineToEdit.value) return
  const index = machines.value.findIndex((x) => x.machine_id === machineToEdit.value!.machine_id)
  machines.value[index].isSelected = !machines.value[index].isSelected
  machines.value[index].selectedPort = port
  // Toggling us back to the normal page
  machineToEdit.value = null
}

onMounted(async () => {
  await invalidateMachines()
})

async function invalidateMachines() {
  machines.value = await fetchInterfaces()
}

const onClear = (machine: LocalMachine) => {
  const index = machines.value.findIndex((x) => x.machine_id === machine.machine_id)
  machines.value[index].isSelected = false
  machines.value[index].selectedPort = undefined
}

const onInitialSelection = (machine: LocalMachine) => {
  const machineRef = machines.value.find((x) => x.machine_id === machine.machine_id)
  if (!machineRef) throw new Error('Could not find machine')
  machineToEdit.value = machineRef
}

async function saveChanges() {
  const activeContextId = contextStore.activeContext?.id
  if (!activeContextId) throw new Error('No active context ID found')

  setIsLoading(true)

  await addMachinesToNetwork(activeContextId, props.network.name, machinesForUpdating.value)
  await contextStore.invalidateContextById(activeContextId)
  await invalidateMachines()

  setIsLoading(false)

  closeModal()
}

const machinesForUpdating = computed(
  () =>
    machines.value
      .map((x) => ({
        machineName: x.name,
        port: x.selectedPort,
      }))
      .filter((x) => !!x.port) as { machineName: string; port: string }[],
)
</script>

<template>
  <div class="flex items-center justify-center">
    <button @click="openModal" class="app-btn-secondary-small">Add Machine</button>
  </div>

  <TransitionRoot appear :show="isOpen" as="template">
    <Dialog as="div" class="relative" @close="closeModal">
      <!-- Backdrop -->
      <div class="bg-(--app-bg)/40 fixed inset-0 backdrop-blur-lg transition-opacity" />

      <div class="fixed inset-0 overflow-y-auto">
        <div class="flex min-h-full items-center justify-center p-4 text-center">
          <TransitionChild
            as="template"
            enter="ease-out duration-300"
            enter-from="opacity-0 scale-95"
            enter-to="opacity-100 scale-100"
            leave="ease-in duration-200"
            leave-from="opacity-100 scale-100"
            leave-to="opacity-0 scale-95"
          >
            <DialogPanel
              class="app-card p-3! rounded-2xl! flex min-h-[312px] min-w-[512px] flex-col items-start gap-6"
            >
              <div class="flex w-full flex-col items-start gap-6">
                <h6>Network Mangement</h6>
              </div>

              <div v-if="!machineToEdit" class="flex w-full flex-col items-start">
                <h6>Select Machines for Network</h6>
                <NetworkMachineList
                  @selected-change="onInitialSelection"
                  :machine-list="machines.filter((x) => !x.isSelected)"
                  :disabled="false"
                ></NetworkMachineList>
              </div>

              <div v-else class="flex w-full flex-col items-start">
                <h6 class="mr-auto">Select A Port For "{{ machineToEdit.name }}"</h6>
                <div class="flex w-full flex-col items-start gap-3">
                  <div
                    class="border-(--app-primary-border) mt-3 flex min-h-[128px] w-full flex-1 flex-col overflow-y-auto rounded-md border-[1px]"
                  >
                    <div v-for="item of machineToEdit.ports">
                      <div
                        @click="() => onSelectionWithPort(item)"
                        class="border-(--app-secondary-border) flex h-[48px] w-full shrink-0 items-center justify-between border-b-[1px] px-3"
                      >
                        <div class="flex items-center gap-3">
                          <h6>{{ item }}</h6>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
              <div class="flex w-full gap-3 overflow-x-auto">
                <div
                  @click="() => onClear(item)"
                  v-for="item of machines.filter((x) => x.isSelected)"
                  :key="item.machine_id"
                >
                  <div class="app-flag-dark flex items-center gap-3">
                    <span>{{ item.name }} </span>
                  </div>
                </div>
              </div>
              <div class="grid w-full grid-cols-2 gap-3">
                <button @click="closeModal" class="app-btn-secondary w-full">Cancel</button>
                <button
                  :disabled="!machinesForUpdating?.length"
                  @click="saveChanges"
                  class="app-btn-primary w-full"
                >
                  Save
                </button>
              </div>
            </DialogPanel>
          </TransitionChild>
        </div>
      </div>
    </Dialog>
  </TransitionRoot>
</template>
