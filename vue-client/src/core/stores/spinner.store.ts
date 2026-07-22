import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useSpinnerStore = defineStore('spinner', () => {
  const isLoading = ref(false)

  function setIsLoading(value: boolean) {
    isLoading.value = value
  }

  return {
    isLoading,
    setIsLoading,
  }
})
