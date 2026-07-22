<script setup lang="ts">
import { useApiUrl } from '@/core/plugins/apiUrlPlugin'
import { useImagesStore } from '@/core/stores/images-store'
import { watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'

const route = useRoute()
const router = useRouter()

const { setApiKey } = useApiUrl()

const imageStore = useImagesStore()

watch(
  () => route.params.inviteUrl,
  (newId, _) => {
    setApiKey(newId as string)
    // When changing context APIs, we don't want to show stale images from prev. context
    imageStore.clearImageCache()
    router.push({ name: 'home' })
  },
  {
    immediate: true,
  },
)
</script>

<template></template>
