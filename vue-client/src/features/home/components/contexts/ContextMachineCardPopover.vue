<script setup lang="ts">
import { type LocalMachine } from '@/shared/types/contexts.model'
import { useContextStore } from '@/core/stores/context-store'
import { Popover, PopoverButton, PopoverPanel } from '@headlessui/vue'
import FadeInOut from '@/shared/ui/transitions/FadeInOut.vue'

const props = defineProps<{
  machine: LocalMachine
}>()

const getMachineColor = (item: LocalMachine) => {
  if (item.powerState === 'on') {
    return 'text-(--app-success)'
  } else if (item.powerState === 'off') {
    return 'text-(--app-danger)'
  } else {
    return 'text-(--app-secondary-text)'
  }
}

const contextsStore = useContextStore()
</script>

<template>
  <Popover class="relative flex items-center justify-center">
    <PopoverButton class="outline-none">
      <svg
        xmlns="http://www.w3.org/2000/svg"
        fill="none"
        viewBox="0 0 24 24"
        stroke-width="1.0"
        stroke="currentColor"
        class="text-(--app-secondary-text) hover:text-(--app-primary-text) size-7 outline-none transition duration-200"
      >
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          d="M12 6.75a.75.75 0 1 1 0-1.5.75.75 0 0 1 0 1.5ZM12 12.75a.75.75 0 1 1 0-1.5.75.75 0 0 1 0 1.5ZM12 18.75a.75.75 0 1 1 0-1.5.75.75 0 0 1 0 1.5Z"
        />
      </svg>
    </PopoverButton>

    <FadeInOut>
      <PopoverPanel
        class="text-(--app-secondary-text) app-card absolute left-3 top-3 z-[5] w-[96px]"
      >
        <div class="flex flex-col items-center justify-center p-3">
          <button
            :disabled="!props.machine.activeImage"
            :class="!props.machine.activeImage && 'opacity-30'"
            class="duration-300 hover:-translate-y-0.5 hover:brightness-125"
            @click="async () => await contextsStore.toggleMachinePower(props.machine.name)"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 24 24"
              stroke-width="1.5"
              stroke="currentColor"
              class="size-5"
              :class="getMachineColor(props.machine)"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                d="M5.636 5.636a9 9 0 1 0 12.728 0M12 3v9"
              />
            </svg>
          </button>
        </div>
      </PopoverPanel>
    </FadeInOut>
  </Popover>
</template>
