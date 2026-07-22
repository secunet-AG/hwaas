<script setup lang="ts">
import { onErrorCaptured } from 'vue'
import { useToasterStore } from '../stores/toaster.store'

const toasterStore = useToasterStore()

onErrorCaptured((e) => {
  console.error(e)
  toasterStore.createAlert(e.name, {
    message: e.message,
    duration: 'long',
    severity: 'error',
  })
})
</script>
<template>
  <slot></slot>
</template>
