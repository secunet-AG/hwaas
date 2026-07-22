<script setup lang="ts">
import { useRoute, useRouter } from 'vue-router'
import { watch, computed, ref } from 'vue'
import { useContextStore } from '@/core/stores/context-store'
import MachineImageSelector from './components/MachineImageSelector.vue'
import { useDrivesApi } from '@/shared/lib/api/drives/drives.api'
import { useMachinesApi } from '@/shared/lib/api/machines/machines.api'
import type { LocalImage } from '@/shared/types/images.model'
import type { LocalMachine } from '@/shared/types/contexts.model'

const route = useRoute()

const router = useRouter()

const { createDrive } = useDrivesApi()
const { enableUsb } = useMachinesApi()

const machineId = computed(() => route.params.machineId as string)

const machine = computed(() => {
  const activeMachine = contextStore.activeContext?.machines.find(
    (x) => x.machine_id == machineId.value,
  )
  return activeMachine ?? (null as LocalMachine | null)
})

const pristine = computed(() => JSON.stringify(machine.value))

const isDirty = computed(() => {
  if (!selectedImage.value || !machine.value) return false

  const newMachine: LocalMachine = { ...machine.value, activeImage: selectedImage.value }

  return JSON.stringify(newMachine) !== pristine.value
})

const contextStore = useContextStore()

const selectedImage = ref<LocalImage | null>(null)

async function onSave() {
  if (!machine.value) throw new Error('No machine found')
  if (!selectedImage.value) throw new Error('No selected image found')
  if (!contextStore.activeContext) throw new Error('No active context')
  if (!isDirty.value) return

  const defaultDriveName = crypto.randomUUID()

  let createDriveRes = await createDrive(
    contextStore.activeContext.id,
    defaultDriveName,
    selectedImage.value.image_hash,
  )

  if (createDriveRes.error) {
    console.error(createDriveRes.error)
    return
  }

  let enableUsbRes = await enableUsb(
    contextStore.activeContext.id,
    machine.value?.name,
    defaultDriveName,
  )

  if (enableUsbRes.error) {
    console.error(enableUsbRes.error)
    return
  }

  const updatedMachine = {
    ...machine.value,
    activeImage: selectedImage.value,
  }

  await contextStore.updateMachineLocally(machine.value.name, updatedMachine)

  router.push({ name: 'home' })
}

async function onImageSelection(image: LocalImage) {
  selectedImage.value = image
}

async function togglePower() {
  const machineName = machine.value?.name
  if (!machineName) throw new Error('Could not find machineName')
  await contextStore.toggleMachinePower(machineName)
}

const getMachineColor = (item: LocalMachine) => {
  if (item.powerState === 'on') {
    return 'text-(--app-success)'
  } else if (item.powerState === 'off') {
    return 'text-(--app-danger)'
  } else {
    return 'text-(--app-secondary-text)'
  }
}
</script>

<template>
  <div v-if="machine" class="grid h-full w-full grid-rows-[1fr_96px]">
    <div class="flex h-full w-full flex-col items-center overflow-y-auto pb-6 sm:items-start">
      <div class="flex w-full items-center justify-between">
        <h4 class="mr-auto text-2xl">Machine Edit</h4>
        <button
          class="mb-auto duration-300 hover:-translate-y-0.5 hover:brightness-125"
          :class="!machine.activeImage && 'opacity-30'"
          :disabled="!machine.activeImage"
          @click="togglePower"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            fill="none"
            viewBox="0 0 24 24"
            stroke-width="1.5"
            stroke="currentColor"
            class="size-8"
            :class="getMachineColor(machine)"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              d="M5.636 5.636a9 9 0 1 0 12.728 0M12 3v9"
            />
          </svg>
        </button>
      </div>

      <div
        class="border-(--app-secondary-border) mt-6 flex w-full flex-col gap-6 border-t-[1px] pt-6 sm:grid sm:grid-cols-2"
      >
        <div class="hidden flex-col sm:flex">
          <h6 class="text-xl">Machine Metadata</h6>
          <span class="text-(--app-secondary-text)/80"
            >View your machine, and configure its power and drives</span
          >
        </div>
        <div class="flex flex-col">
          <label for="context-name">Machine Image</label>
          <MachineImageSelector
            v-bind:active-machine="machine"
            @on-image-selection="async (x) => await onImageSelection(x)"
            class="mt-3"
          />
        </div>
      </div>
    </div>
    <div class="mt-auto flex w-full justify-center pb-3">
      <div class="grid w-full max-w-[1200px] grid-cols-2 items-center gap-6 md:gap-12">
        <button
          @click="
            () => {
              router.push({ name: 'home' })
            }
          "
          class="app-btn-secondary w-full"
        >
          Cancel
        </button>
        <button @click="onSave" class="app-btn-primary w-full" :disabled="!isDirty">
          Save Changes
        </button>
      </div>
    </div>

    <!-- <div class="grid w-full grid-cols-2 gap-6 pt-6">
      <button class="app-btn-primary flex-1">Save</button>
      <button class="app-btn-secondary flex-1">Cancel</button>
    </div> -->
  </div>
  <div v-else class="flex h-full w-full items-center justify-center gap-1">
    <h6>Machine Not Found</h6>
    <span
      >Please try another page, if you are confident this machine exists, please contact an
      administrator.</span
    >
  </div>
</template>
