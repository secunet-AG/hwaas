<script setup lang="ts">
import { computed, nextTick, onMounted, ref } from 'vue'
import { TransitionRoot, TransitionChild, Dialog, DialogPanel } from '@headlessui/vue'
import { useApiUrl } from '@/core/plugins/apiUrlPlugin'
import type { LocalMachine } from '@/shared/types/contexts.model'
import NetworkMachineList from './NetworkMachineList.vue'
import { useMachinesApi } from '@/shared/lib/api/machines/machines.api'
import { useContextStore } from '@/core/stores/context-store'
import { useNetworksApi } from '@/shared/lib/api/networks/networks.api'
import type { MachineWithPort } from '@/shared/types/contexts.model'
import { useSpinnerStore } from '@/core/stores/spinner.store'
import { nameGenerator } from '@/shared/lib/etc/machine-name-generator'
import Tooltip from '@/shared/ui/tooltip/Tooltip.vue'

const { createNetwork, patchNetwork } = useNetworksApi()

const { getNetworkInterfaces } = useMachinesApi()

const contextsStore = useContextStore()

const { setIsLoading } = useSpinnerStore()

const networkName = ref<string | null>(null)

const machineToEdit = ref<MachineWithPort | null>(null)

const isOpen = ref(false)

const machines = ref<MachineWithPort[]>([])

async function closeModal() {
  networkName.value = null
  await invalidateMachines()
  isOpen.value = false
}
function openModal() {
  isOpen.value = true
}

async function fetchInterfaces(): Promise<MachineWithPort[]> {
  let selectedMachines = contextsStore.activeContext?.machines

  if (!selectedMachines) throw new Error('No machines selected on context')

  selectedMachines = selectedMachines.map((x) => ({ ...x, isSelected: false }))

  if (!contextsStore?.activeContext?.id) throw new Error('No active context')

  const machinesRes = await Promise.all(
    selectedMachines.map(async (x) => {
      const res = await getNetworkInterfaces(contextsStore!.activeContext!.id, x.name)
      return [x, res] as const
    }),
  )

  const awaitedMachines = machinesRes.filter(([x, res]) => res.data && !res.error)

  return awaitedMachines.map(([x, res]) => ({ ...x, isSelected: false, ports: res.data ?? [] }))
}

onMounted(async () => {
  await invalidateMachines()
})

async function invalidateMachines() {
  machines.value = await fetchInterfaces()
}

function onGenerateContextName() {
  networkName.value = nameGenerator()
}

const onSelectionWithPort = (port: string) => {
  if (!machineToEdit.value) return
  const index = machines.value.findIndex((x) => x.machine_id === machineToEdit.value!.machine_id)
  machines.value[index].isSelected = !machines.value[index].isSelected
  machines.value[index].selectedPort = port
  // Toggling us back to the normal page
  machineToEdit.value = null
}

const onInitialSelection = (machine: LocalMachine) => {
  const machineRef = machines.value.find((x) => x.machine_id === machine.machine_id)
  if (!machineRef) throw new Error('Could not find machine')
  machineToEdit.value = machineRef
}

const onClear = (machine: LocalMachine) => {
  const index = machines.value.findIndex((x) => x.machine_id === machine.machine_id)
  machines.value[index].isSelected = false
  machines.value[index].selectedPort = undefined
}

async function onSubmit() {
  setIsLoading(true)
  const activeContextId = contextsStore.activeContext?.id
  if (!activeContextId || !networkName.value)
    throw new Error('Cannot create network with current information.')
  await createNetwork(activeContextId, networkName.value, payload.value)
  await contextsStore.invalidateContextById(activeContextId)
  await invalidateMachines()

  await nextTick()
  setIsLoading(false)
  await closeModal()
}

const readyToSubmit = computed(() => machines.value.filter((x) => x.isSelected && x.ports))

const payload = computed(() =>
  readyToSubmit.value.reduce<Record<string, Record<string, any>>>(
    (acc, cur) => ({ ...acc, [cur.name]: { [cur.selectedPort as string]: {} } }),
    {},
  ),
)
</script>

<template>
  <div class="flex items-center justify-center">
    <button @click="openModal" class="app-btn-secondary-small">Add Network</button>
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
                <div class="flex w-full flex-col items-start gap-1">
                  <div class="flex gap-1">
                    <label>Network Name</label>

                    <Tooltip
                      :options="{
                        message: 'Generate a name',
                        xOffsetOverride: 32,
                        yOffsetOverride: -12,
                      }"
                    >
                      <button
                        @click="onGenerateContextName"
                        class="text-(--app-secondary-text)/60 hover:text-(--app-primary-text) flex h-full items-center transition-all"
                      >
                        <svg
                          width="24"
                          height="24"
                          fill="none"
                          stroke="currentColor"
                          stroke-width="1.5"
                          viewBox="0 0 24 24"
                          stroke-linecap="round"
                        >
                          <path
                            d="M8.25 8h-.5m4.5 4h-.5m4.5 4h-.5M3 9.4c0-2.24 0-3.36.436-4.216a4 4 0 0 1 1.748-1.748C6.04 3 7.16 3 9.4 3h5.2c2.24 0 3.36 0 4.216.436a4 4 0 0 1 1.748 1.748C21 6.04 21 7.16 21 9.4v5.2c0 2.24 0 3.36-.436 4.216a4 4 0 0 1-1.748 1.748C17.96 21 16.84 21 14.6 21H9.4c-2.24 0-3.36 0-4.216-.436a4 4 0 0 1-1.748-1.748C3 17.96 3 16.84 3 14.6z"
                          ></path>
                        </svg>
                      </button>
                    </Tooltip>
                  </div>
                  <input
                    v-model="networkName"
                    class="app-input w-full"
                    placeholder="e.g Production Network One"
                  />
                </div>

                <hr class="border-b-(--app-secondary-border) w-full border-[1px]" />

                <div v-if="!machineToEdit" class="flex w-full flex-col items-start">
                  <h6>Select Machines for Network</h6>

                  <NetworkMachineList
                    @selected-change="onInitialSelection"
                    :machine-list="machines.filter((x) => !x.isSelected)"
                    :disabled="false"
                  ></NetworkMachineList>
                </div>
                <div class="flex w-full flex-col items-start" v-else>
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
                    class="cursor-pointer"
                    :key="item.machine_id"
                  >
                    <div class="app-flag-dark flex items-center gap-3">
                      <span class="text-(--app-secondary-text)!"
                        >{{ `${item.name} | ${item.selectedPort}` }}
                      </span>
                    </div>
                  </div>
                </div>
                <div class="grid w-full grid-cols-2 gap-3">
                  <button @click="closeModal" class="app-btn-secondary w-full">Cancel</button>
                  <button
                    :disabled="!(readyToSubmit.length && networkName)"
                    @click="onSubmit"
                    class="app-btn-primary w-full"
                  >
                    Create
                  </button>
                </div>
              </div>
            </DialogPanel>
          </TransitionChild>
        </div>
      </div>
    </Dialog>
  </TransitionRoot>
</template>
