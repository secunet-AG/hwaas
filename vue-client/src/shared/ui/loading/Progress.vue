<script setup lang="ts">
const completionPercentage = defineModel<number>()
</script>

<template>
  <div class="bg-(--app-secondary-border) w-full rounded-2xl">
    <div
      :style="{
        width: `${completionPercentage}%`,
      }"
      class="h-1 rounded-2xl bg-white transition-all"
    ></div>
  </div>
</template>
