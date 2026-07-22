import { ref } from 'vue'

export function useModal() {
  const isOpen = ref(false)

  function closeModal() {
    isOpen.value = false
  }

  function openModal() {
    isOpen.value = true
  }

  return {
    isOpen,
    openModal,
    closeModal,
  }
}
