<script setup lang="ts">
import { ref } from 'vue'

const emit = defineEmits<{
  (event: 'searchEvent', payload: string): void
}>()

const searchValue = ref('')
</script>

<template>
  <div
    class="border-(--app-secondary-border) relative flex w-full items-center rounded-lg border-[1px] pl-2"
  >
    <svg
      xmlns="http://www.w3.org/2000/svg"
      fill="none"
      viewBox="0 0 24 24"
      stroke-width="1.5"
      stroke="currentColor"
      class="text-(--app-secondary-text) size-5"
    >
      <path
        stroke-linecap="round"
        stroke-linejoin="round"
        d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z"
      />
    </svg>

    <input
      placeholder="Search..."
      v-model="searchValue"
      @input="emit('searchEvent', searchValue)"
      class="placeholder:text-(--app-secondary-text) text-(--app-primary-text) w-full min-w-[128px] max-w-[256px] pl-2"
    />
  </div>
</template>
