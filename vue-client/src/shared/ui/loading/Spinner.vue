<script setup lang="ts">
import { useSpinnerStore } from '@/core/stores/spinner.store'
import { storeToRefs } from 'pinia'

const useSpinner = useSpinnerStore()

const spinnerRef = storeToRefs(useSpinner)
</script>

<template>
  <div
    v-if="spinnerRef.isLoading.value"
    class="pointer-events-auto fixed inset-0 z-[999999] flex items-center justify-center backdrop-blur-sm"
  >
    <span class="loader pointer-events-none"></span>
  </div>
</template>

<style>
.loader {
  width: 128px;
  height: 128px;
  border-radius: 50%;
  display: inline-block;
  border-top: 3px solid #fff;
  border-right: 3px solid transparent;
  box-sizing: border-box;
  animation: rotation 1s linear infinite;
}

@keyframes rotation {
  0% {
    transform: rotate(0deg);
  }
  100% {
    transform: rotate(360deg);
  }
}
</style>
