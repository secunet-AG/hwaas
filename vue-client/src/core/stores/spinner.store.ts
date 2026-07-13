// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

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
