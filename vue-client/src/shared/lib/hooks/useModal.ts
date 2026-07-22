// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

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
