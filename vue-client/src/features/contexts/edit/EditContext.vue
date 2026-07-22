<script setup lang="ts">
import { useContextStore } from '@/core/stores/context-store'
import type { Context } from '@/shared/types/contexts.model'
import CreateContext from '../components/context-creation/CreateContext.vue'
import Loading from '@/shared/ui/loading/Loading.vue'
import { ref, watch, computed } from 'vue'
import { useRoute } from 'vue-router'

const route = useRoute()

const contextStore = useContextStore()

async function fetchContextById(id: string) {
  return contextStore.getContext(id)
}

const contextId = computed(() => route.params.contextId as string)

const context = ref<Context | null>(null)

watch(
  contextId,
  async (newId) => {
    const newContext = await fetchContextById(newId)
    if (!newContext) return

    context.value = newContext
  },
  { immediate: true },
)
</script>

<template>
  <CreateContext v-if="context" :existing-context="context ?? undefined" :isUpdating="true" />
  <Loading v-else />
</template>
