<script setup lang="ts">
import { onMounted, ref } from 'vue'
import ContextListPopover from './components/ContextListPopover.vue'
import { useContextStore } from '@/core/stores/context-store'
import { useRouter } from 'vue-router'
import NoContextsFound from './components/context-creation/NoContextsFound.vue'
import { storeToRefs } from 'pinia'
import Tooltip from '@/shared/ui/tooltip/Tooltip.vue'

const contextStore = useContextStore()

const { contexts } = storeToRefs(contextStore)

const router = useRouter()

const isEditing = ref(!contexts.value.length)

async function onContextDelete(id: string) {
  await contextStore.deleteContextById(id)
  if (contexts.value.length === 0) {
    isEditing.value = true
  }
}

function onContextNavToEdit(id: string) {
  router.push({
    name: 'context-edit',
    params: {
      contextId: id,
    },
  })
}

function toggleIsEditing() {
  router.push({ name: 'createContext' })
}
</script>

<template>
  <NoContextsFound v-if="!contexts.length" />
  <div v-else class="flex h-full w-full flex-col">
    <div class="flex items-center justify-between">
      <h2 class="text-2xl">Contexts</h2>
      <Tooltip
        :options="{ message: 'Create a new context', xOffsetOverride: -208, yOffsetOverride: 0 }"
      >
        <button
          @click="toggleIsEditing"
          class="border-(--app-primary-text) flex h-12 w-12 items-center justify-center rounded-full border-[1px]"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 24 24"
            fill="currentColor"
            class="text-(--app-primary-text) size-8"
          >
            <path
              fill-rule="evenodd"
              d="M12 3.75a.75.75 0 0 1 .75.75v6.75h6.75a.75.75 0 0 1 0 1.5h-6.75v6.75a.75.75 0 0 1-1.5 0v-6.75H4.5a.75.75 0 0 1 0-1.5h6.75V4.5a.75.75 0 0 1 .75-.75Z"
              clip-rule="evenodd"
            />
          </svg>
        </button>
      </Tooltip>
    </div>
    <div class="flex flex-col gap-3 pt-6">
      <div v-for="item of contexts" class="app-card flex w-full items-center justify-between">
        <div
          @click="
            () => {
              contextStore.setActiveContext(item.id)
              router.push({ name: 'home' })
            }
          "
          class="flex flex-1 cursor-pointer flex-col"
        >
          <h6>{{ item.name }}</h6>
          <span class="text-(--app-secondary-text) text-sm">{{
            `${item.machines.length} ${item.machines.length === 1 ? 'Machine' : 'Machines'} Active`
          }}</span>
        </div>
        <ContextListPopover
          @edit-event="onContextNavToEdit(item.id)"
          @delete-event="onContextDelete(item.id)"
        />
      </div>
    </div>
  </div>
</template>
