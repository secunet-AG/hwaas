<script setup lang="ts">
import MachineList from './MachineList.vue'
import { useContextStore } from '@/core/stores/context-store'
import type {
  Context,
  LocalMachine,
  ContextConfigurationMachine,
  InventoryMachine,
} from '@/shared/types/contexts.model'
import { computed, onMounted, reactive, useTemplateRef } from 'vue'
import UploadContext from './UploadContext.vue'
import { nameGenerator } from '@/shared/lib/etc/machine-name-generator'
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import SearchForExistingContext from './SearchForExistingContext.vue'
import Tooltip from '@/shared/ui/tooltip/Tooltip.vue'

const router = useRouter()

const contextNameTemplateRef = useTemplateRef('contextName')

onMounted(() => {
  contextNameTemplateRef.value?.focus()
})

const props = defineProps<{
  isUpdating: boolean
  existingContext?: Context
}>()

const DEFAULT_CONTEXT_LIFE = 60 * 60 * 24

const existingContextId = ref<string | null>(null)

const contextStore = useContextStore()

const newContext = reactive<Context>(
  props.existingContext
    ? { ...props.existingContext }
    : {
        id: '0',
        name: '',
        lifetime: DEFAULT_CONTEXT_LIFE,
        machines: [],
        networks: [],
      },
)

const isCreateDisabled = computed(() => {
  const isValidNewContext = newContext.name && newContext.machines.length
  const isValidExistingContext = existingContextId.value && newContext.name

  return !(isValidNewContext || isValidExistingContext)
})

function onGenerateContextName() {
  newContext.name = nameGenerator()
}

function onContextUpload(machines: ContextConfigurationMachine) {
  const newMachines = [] as LocalMachine[]

  Object.keys(machines).forEach((k) => {
    newMachines.push({
      name: k,
      machine_id: String(machines[k].machine_id),
      platform: machines[k].platform,
      powerState: 'unknown',
      serialPorts: [],
    })
  })

  newContext.machines = newMachines
}

const showNameWarningBorder = computed(
  () =>
    !existingContextId.value &&
    newContext.machines.length &&
    !newContext.name.length &&
    'shadow-[0_8px_30px_rgb(0,0,0,0.12)] shadow-red-300/30 border-red-300/60!',
)

function onMachineSelectionChange(machines: InventoryMachine[]) {
  newContext.machines = machines.map((x) => ({
    name: nameGenerator(),
    machine_id: String(x.machine_id),
    platform: x.properties.platform,
    powerState: 'unknown',
    serialPorts: [],
  }))
}

const isCreatingContext = ref(props.isUpdating)

async function createContext() {
  if (existingContextId.value) {
    await onCreateContextFromExistingId()
    return
  }

  const newId = await contextStore.addContext(newContext)
  contextStore.setActiveContext(newId)
  router.push({ name: 'home' })
}

async function onCreateContextFromExistingId() {
  if (!(existingContextId.value && newContext.name))
    throw new Error('Missing existing context id and context name for creation')
  await contextStore.addContextFromExisting(existingContextId.value, newContext.name)
  contextStore.setActiveContext(existingContextId.value)
  router.push({ name: 'home' })
}

async function updateContext() {
  await contextStore.updateContext(newContext)
  contextStore.setActiveContext(newContext.id)
  router.push({ name: 'home' })
}

function onCancel() {
  isCreatingContext.value = false
  router.push({ name: 'home' })
}
</script>

<template>
  <div class="flex h-full w-full flex-col">
    <div class="flex h-full w-full flex-col items-center overflow-y-auto px-6 pb-6 sm:items-start">
      <h4 class="mr-auto text-2xl">Create a new context</h4>
      <div
        class="border-(--app-secondary-border) mt-6 flex w-full flex-wrap gap-6 border-t-[1px] pt-6"
      >
        <div class="hidden min-w-[420px] flex-1 flex-col sm:flex">
          <h6 class="text-xl">Context Metadata</h6>
          <span class="text-(--app-secondary-text)/80"
            >Name your context for future reference (stored locally)</span
          >
        </div>
        <div class="flex min-w-[420px] flex-1 flex-col">
          <div class="flex items-center gap-2">
            <label for="context-name">Context Name</label>
            <Tooltip
              :options="{ message: 'Generate a name', xOffsetOverride: 32, yOffsetOverride: -12 }"
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
            placeholder="e.g MyContext Prod"
            class="app-input mt-3 shrink-0"
            :class="showNameWarningBorder"
            required="true"
            id="context-name"
            ref="contextName"
            :value="newContext.name"
            @input="
              (e: any) => {
                newContext.name = e.target.value
              }
            "
          />
        </div>
      </div>
      <div
        class="border-(--app-secondary-border) mt-6 flex w-full flex-wrap gap-6 border-b-[1px] border-t-[1px] pb-6 pt-6"
      >
        <div class="hidden min-w-[420px] flex-1 flex-col sm:flex">
          <h6 class="text-xl">Machine Selection</h6>
          <span class="text-(--app-secondary-text)/80"
            >Select the machines or upload a context file for machine selection</span
          >
        </div>
        <div class="min-w-[420px] flex-1">
          <div class="flex w-full flex-col items-center gap-3">
            <div class="w-full">
              <UploadContext
                :disabled="props.isUpdating || !!existingContextId"
                @context-upload="onContextUpload"
                @context-upload-fail="
                  () => {
                    console.error('Failed context creation via upload!')
                  }
                "
              />
            </div>

            <div class="grid w-full grid-cols-[1fr_32px_1fr] items-center gap-3">
              <hr class="border-(--app-secondary-border) border-b-[1px]" />
              <div class="flex justify-center">
                <span class="text-(--app-secondary-text)">or</span>
              </div>
              <hr class="border-(--app-secondary-border) border-b-[1px]" />
            </div>

            <div class="flex w-full flex-col">
              <label>Select Machines</label>
              <MachineList
                :disabled="props.isUpdating || !!existingContextId"
                @selected-change="onMachineSelectionChange"
              />
            </div>
          </div>
        </div>
      </div>
      <div
        class="border-(--app-secondary-border) mr-auto mt-6 flex w-full flex-wrap gap-6 border-b-[1px] pb-6"
      >
        <div class="hidden min-w-[420px] flex-1 flex-col sm:flex">
          <h6 class="text-xl">Existing Context UUID</h6>
          <span class="text-(--app-secondary-text)/80"
            >Override the above settings and select an existing context</span
          >
        </div>
        <SearchForExistingContext
          class="min-w-[420px] flex-1"
          @search-cleared="
            () => {
              existingContextId = null
            }
          "
          @existing-context-event="
            (x) => {
              existingContextId = x
            }
          "
          :is-disabled="props.isUpdating || !!newContext.machines.length"
        ></SearchForExistingContext>
      </div>
    </div>
    <div class="grid w-full grid-cols-2 gap-6 pt-6">
      <button @click="onCancel" class="app-btn-secondary flex-1">Cancel</button>
      <Tooltip
        class="flex-1"
        v-if="!isUpdating"
        :options="{
          message: 'Must create a context name',
          yOffsetOverride: -64,
          isDisabled: !isCreateDisabled,
        }"
      >
        <button
          v-bind:disabled="isCreateDisabled"
          @click="createContext"
          class="app-btn-primary w-full"
        >
          Create a new context
        </button>
      </Tooltip>

      <button v-else class="app-btn-primary flex-1" @click="updateContext">Update Context</button>
    </div>
  </div>
</template>
