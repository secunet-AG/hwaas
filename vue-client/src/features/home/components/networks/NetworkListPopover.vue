<script setup lang="ts">
import FadeInOut from '@/shared/ui/transitions/FadeInOut.vue'
import { Popover, PopoverButton, PopoverPanel } from '@headlessui/vue'
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
        <div class="flex flex-col gap-2">
          <button class="hover:text-(--app-primary-text)">1</button>
          <hr class="border-(--app-secondary-border) border-b-[1px]" />
          <button class="hover:text-(--app-primary-text) border-(--app-secondary-border)">2</button>
        </div>
      </PopoverPanel>
    </FadeInOut>
  </Popover>
</template>
