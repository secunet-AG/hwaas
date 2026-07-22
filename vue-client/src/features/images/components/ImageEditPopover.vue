<script setup lang="ts">
import FadeInOut from '@/shared/ui/transitions/FadeInOut.vue'
import { Popover, PopoverButton, PopoverPanel } from '@headlessui/vue'

const emit = defineEmits<{
  (event: 'deleteEvent'): void
}>()
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
        class="bg-(--app-bg) app-primary-shadow border-(--app-secondary-border) text-(--app-secondary-text) absolute -left-16 right-0 top-8 z-[5] w-[96px] rounded-md border-[1px]"
      >
        <div class="flex flex-col">
          <button
            @click="() => emit('deleteEvent')"
            class="hover:text-(--app-primary-text) border-(--app-secondary-border) flex items-center justify-center border-b-[1px] p-3"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 24 24"
              stroke-width="1.5"
              stroke="currentColor"
              class="text-(--app-danger) size-6"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                d="m14.74 9-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 0 1-2.244 2.077H8.084a2.25 2.25 0 0 1-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 0 0-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 0 1 3.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 0 0-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 0 0-7.5 0"
              />
            </svg>
          </button>
        </div>
      </PopoverPanel>
    </FadeInOut>
  </Popover>
</template>
