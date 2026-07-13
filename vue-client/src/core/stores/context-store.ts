// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

import { useContextsApi } from '@/shared/lib/api/contexts/contexts.api'
import { useMachinesApi } from '@/shared/lib/api/machines/machines.api'
import type { FetchResult } from '@/shared/lib/api/safeFetch'
import type { Context, LocalMachine } from '@/shared/types/contexts.model'
import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'

const CONTEXT_LOCALSTORE_KEY = 'context_local_key'
const CONTEXT_HISTORY_KEY = 'context_history_key'
const MAXIMUM_CONTEXT_LIFETIME_IN_SECONDS = +import.meta.env
  .VITE_MAXIMUM_CONTEXT_LIFETIME_IN_SECONDS

export const useContextStore = defineStore('context', () => {
  const _initial = initializeContexts()
  const contexts = ref<Context[]>(_initial.contexts)
  const activeContextIndex = ref<number>(_initial.activeContextIndex)

  const { deleteContext, getContextFromId, reserveContext, setContextLifetime } = useContextsApi()
  const { powerOnMachine, powerOffMachine, invalidateMachines } = useMachinesApi()

  const activeContext = computed<Context | null>(
    () => contexts.value[activeContextIndex.value] ?? null,
  )
  const activeMachines = computed<LocalMachine[]>(() => activeContext.value?.machines ?? [])

  async function getContext(id: string) {
    return contexts.value.find((x) => x.id === id)
  }

  async function setActiveContext(id: string | number) {
    const newIndex = contexts.value.findIndex((x) => x.id == id)
    await invalidateContext(contexts.value[newIndex])
    activeContextIndex.value = newIndex
  }

  async function addContext(newContext: Context) {
    const { data: reservationResponse, error } = await reserveContext(newContext.machines)
    if (!reservationResponse || error) {
      throw new Error('Could not add context')
    }
    contexts.value = [...contexts.value, { ...newContext, id: reservationResponse }]
    const historyStorage = localStorage.getItem(CONTEXT_HISTORY_KEY)
    const oneWeekAgo = Date.now() - 3600 * 24 * 7

    let payload = [] as { contextId: string; exp: number }[]
    try {
      payload = JSON.parse(historyStorage ?? '[]')
    } catch {}

    const filteredPayload = payload.filter((x) => x.exp > oneWeekAgo)
    const newItem = {
      contextId: reservationResponse,
      exp: Date.now() + MAXIMUM_CONTEXT_LIFETIME_IN_SECONDS,
    }

    localStorage.setItem(CONTEXT_HISTORY_KEY, JSON.stringify([...filteredPayload, newItem]))

    return reservationResponse
  }

  async function deleteContextById(contextId: string) {
    await deleteContext(contextId)

    const deletionIndex = contexts.value.findIndex((x) => x.id === contextId)
    if (deletionIndex <= activeContextIndex.value) {
      activeContextIndex.value -= 1
    }

    contexts.value = contexts.value.filter((x) => x.id !== contextId)
  }

  async function updateContext(newContext: Context) {
    const index = contexts.value.findIndex((x) => x.id === newContext.id)
    if (index < 0) return

    contexts.value[index] = { ...newContext }
  }

  async function updateMachineLocally(machineName: string, newMachine: LocalMachine) {
    if (!activeContext.value) throw new Error('No active context!')
    const idx = activeContext.value.machines.findIndex((x) => x.name === machineName)
    if (idx === -1) throw new Error('Machine not found!')

    const newCopy = [...activeContext.value.machines]
    newCopy[idx] = newMachine

    activeContext.value.machines = newCopy
  }

  async function toggleMachinePower(machineName: string) {
    try {
      if (!activeContext.value) throw new Error('No active context')
      const machineIndex = activeContext.value.machines.findIndex((x) => x.name === machineName)
      const machine = { ...activeContext.value.machines[machineIndex] }
      if (!machine) throw new Error('Could not find machine')

      // Toggle the machine power state. If we get an error, set it to unknown.
      let res =
        machine.powerState == 'on'
          ? await powerOffMachine(activeContext.value.id, machineName)
          : await powerOnMachine(activeContext.value.id, machineName)

      if (!res.error && res.data) {
        // Flatten and see if any response if true
        const isOn = res.data.findIndex((x) => x.state) != -1
        machine.powerState = isOn ? 'on' : 'off'
      } else {
        machine.powerState = 'unknown'
      }

      activeContext.value.machines[machineIndex] = machine
    } catch {
      const machine = activeContext.value?.machines.find((x) => x.name === machineName)
      if (!machine) throw new Error('Could not find machine')
      machine.powerState = 'unknown'
    }
  }

  async function addContextFromExisting(id: string, name: string) {
    if (contexts.value.find((x) => x.id === id)) {
      throw new Error('Context already exists')
    }
    const { data, error } = await getContextFromId(id)

    if (!data) {
      if (error) {
        console.error('Error thrown when fetching context from id: ', error)
        return
      } else {
        console.error('No data returned from context search by id')
        return
      }
    }

    contexts.value = [...contexts.value, { ...data, name }]
  }

  async function retryWithBackoff<T>(
    fn: () => Promise<T>,
    {
      retries = 5,
      baseDelay = 300, // time in ms
      maxDelay = 5000,
    } = {},
  ): Promise<T> {
    let attempt = 0

    while (true) {
      try {
        return await fn()
      } catch (err) {
        attempt++
        if (attempt > retries) throw err
        // Interpolate between the two delay times, add some jitter as well
        const delay = Math.min(maxDelay, baseDelay * Math.pow(2, attempt) + Math.random() * 200)

        await new Promise((resolve) => setTimeout(resolve, delay))
      }
    }
  }

  let invalidationInFlight: Promise<void> | null = null

  async function invalidateContextsCache() {
    if (!invalidationInFlight) {
      invalidationInFlight = retryWithBackoff(
        async () => {
          const updated = await invalidateCachedContexts(contexts.value)
          contexts.value = updated
        },
        { retries: 3, baseDelay: 120, maxDelay: 1600 },
      )
        .catch((e) => {
          console.error(e)
          contexts.value = []
        })
        .finally(() => {
          invalidationInFlight = null
        })
    }

    return invalidationInFlight
  }

  async function invalidateContextById(contextId: string) {
    const ctxIndex = contexts.value.findIndex((x) => x.id === contextId)
    if (ctxIndex == -1) throw new Error('Could not find context')

    const { data: freshContext, error } = await invalidateContext(contexts.value[ctxIndex])

    if (error) {
      throw new Error(
        `An error was thrown when invalidating a context (perhaps it's expired): ${error}`,
      )
    }

    if (freshContext) {
      contexts.value[ctxIndex] = freshContext
    }
  }

  /*
    Increment the context lifetime, the default time is one hour
  */
  async function setLifetime(contextId: string, seconds: number = 3600): Promise<number> {
    const ctx = contexts.value.find((x) => x.id === contextId)
    if (!ctx) throw new Error('Could not find context')
    const { data, error } = await setContextLifetime(ctx, seconds)
    if (error || !data) {
      throw new Error(`Error when setting context lifetime: ${error}`)
    }

    if (data < 400) {
      ctx.lifetime = seconds
    } else {
      console.error('Could not update lifetime')
    }
    return data
  }

  function initializeContexts(): { contexts: Context[]; activeContextIndex: number } {
    const DEFAULT_CONFIGURATION = { contexts: [] as Context[], activeContextIndex: 0 }
    const configuration = localStorage.getItem(CONTEXT_LOCALSTORE_KEY)

    if (!configuration) {
      return DEFAULT_CONFIGURATION
    }

    let { contexts, activeContextIndex } = JSON.parse(configuration)

    if (Array.isArray(contexts) && typeof activeContextIndex === 'number') {
      if (activeContextIndex < 0 || activeContextIndex >= contexts.length) {
        activeContextIndex = 0
      }
      return { contexts, activeContextIndex }
    }

    DEFAULT_CONFIGURATION.contexts.forEach((x) => {
      x.machines.forEach((y) => {
        y.powerState = 'unknown'
      })
    })

    return DEFAULT_CONFIGURATION
  }

  async function invalidateCachedContexts(contexts: Context[]): Promise<Context[]> {
    // Our names only exist locally, so we have to make a map to reapply them
    const idToNameMap = new Map(contexts.map((x) => [x.id, x.name]))

    const contextResults = await Promise.all(contexts.map((x) => invalidateContext(x)))

    const validContextData = contextResults.filter((x) => x.data && !x.error).map((x) => x.data)

    const contextsWithName = validContextData.map((x) => ({
      ...x,
      id: x!.id,
      name: idToNameMap.get(x!.id) ?? '',
    }))

    return contextsWithName as Context[]
  }

  async function invalidateContext(staleContext: Context): Promise<FetchResult<Context>> {
    const { data: context, error: contextError } = await getContextFromId(staleContext.id)
    if (contextError || !context?.id) {
      return {
        data: null,
        error: String(contextError),
      }
    }
    const freshContext = { ...context, name: staleContext.name }
    const { data: machines, error: machinesError } = await invalidateMachines(freshContext)

    if (machinesError) {
      console.error('Error fetching machines: ', machinesError)
    }

    return {
      data: { ...freshContext, machines: machines || [] },
      error: null,
    }
  }

  return {
    contexts,
    activeContextIndex,
    activeContext,
    activeMachines,
    getContext,
    getContextFromId,
    setActiveContext,
    addContext,
    deleteContextById,
    updateContext,
    updateMachineLocally,
    toggleMachinePower,
    addContextFromExisting,
    setLifetime,
    invalidateContextsCache,
    invalidateContextById,
  }
})

export function cacheContexts({
  contexts,
  activeContextIndex,
}: {
  contexts: Context[]
  activeContextIndex: number
}) {
  localStorage.setItem(
    CONTEXT_LOCALSTORE_KEY,
    JSON.stringify({
      contexts: contexts,
      activeContextIndex: activeContextIndex,
    }),
  )
}
