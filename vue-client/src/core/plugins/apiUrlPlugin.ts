// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

import { ref, type Ref, type App, inject, computed } from 'vue'

const API_STORAGE_KEY = 'API_STORAGE_KEY'

export const API_INJECTION_KEY = Symbol('api injection key')

export const apiUrlRef = ref<string>(
  localStorage.getItem(API_STORAGE_KEY) ?? import.meta.env.VITE_API_URL,
)
export type APIUrlInner = {
  apiUrl: Ref<string>
  setApiKey: (newKey: string) => void
}

const apiUrlProvider = (): APIUrlInner => {
  const setApiKey = (newKey: string) => {
    apiUrlRef.value = newKey
    localStorage.setItem(API_STORAGE_KEY, newKey)
  }

  return {
    apiUrl: computed(() => apiUrlRef.value),
    setApiKey,
  }
}

export const apiUrlPlugin = (app: App) => {
  app.provide<APIUrlInner>(API_INJECTION_KEY, apiUrlProvider())
}

export const useApiUrl = () => {
  const injected = inject<APIUrlInner>(API_INJECTION_KEY)
  if (!injected) throw new Error('Could not inject the API URL!')
  return injected
}
