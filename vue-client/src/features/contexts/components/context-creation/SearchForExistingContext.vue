<script setup lang="ts">
import { useDebounceFn } from '@vueuse/core'
import { ref, reactive } from 'vue'
import { useContextStore } from '@/core/stores/context-store'

const { getContextFromId } = useContextStore()

const props = defineProps<{
  isDisabled: boolean
}>()

const emits = defineEmits<{
  (event: 'existingContextEvent', payload: string): void
  (event: 'searchCleared'): void
}>()

const hasExistingContextSearchFailed = ref(false)

const hasExistingContextSearchSucceeded = ref(false)

const searchClassObj = reactive({
  'shadow-[0_8px_30px_rgb(0,0,0,0.12)] shadow-red-300/30 border-red-300/60!':
    hasExistingContextSearchFailed,
  'shadow-[0_8px_30px_rgb(0,0,0,0.12)] shadow-green-300/30 border-green-300/60!':
    hasExistingContextSearchSucceeded,
})

const searchDebounced = useDebounceFn(async (id: string) => {
  try {
    if (!id || !id.length) {
      hasExistingContextSearchSucceeded.value = false
      hasExistingContextSearchFailed.value = false
      emits('searchCleared')
      return
    }
    const { data, error } = await getContextFromId(id)
    if (error) {
      console.error(error)
    }
    if (data) {
      emits('existingContextEvent', data.id)
      hasExistingContextSearchSucceeded.value = true
      hasExistingContextSearchFailed.value = false
    }
  } catch {
    hasExistingContextSearchSucceeded.value = false
    hasExistingContextSearchFailed.value = true
  }
}, 1000)
</script>

<template>
  <div class="flex flex-col">
    <label :class="props.isDisabled ? 'text-(--app-secondary-text)/30!' : ''"
      >Existing Context UUID</label
    >
    <input
      placeholder="e.g aa599862-633a..."
      class="app-input mt-2 w-full shrink-0"
      :class="searchClassObj"
      :disabled="props.isDisabled"
      id="existing-context"
      @input="(e: any) => searchDebounced(e.target.value)"
    />
  </div>
</template>
