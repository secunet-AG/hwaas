// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

import type {
  LocalMachine,
  ContextConfigurationMachine,
  Context,
} from '@/shared/types/contexts.model'
import { useNetworksApi } from '../networks/networks.api'
import { safeFetch, type FetchResult } from '../safeFetch'
import {
  ContextLifetimeSchema,
  ContextMachineSchema,
  ContextMachinesSchema,
  ContextReservationSchema,
} from '@/shared/schemas/contexts.schema'
import type { ApiNetwork } from '@/shared/types/networks.model'
import { useConfig } from '@/core/plugins/config-plugin'

// Wrapping this so we can reuse it in the store and provide the apiURL, which can change
export const useContextsApi = () => {
  const { API_URL } = useConfig()

  const { getAllNetworks } = useNetworksApi()

  async function reserveContext(machines: LocalMachine[]): Promise<FetchResult<string>> {
    const machinesForReservationRecord = {} as ContextConfigurationMachine

    // Convert machines from easy list to the structure needed in the API
    // Note, we would need some logic here for uniqueness for names, maybe using id, etc.

    machines.forEach((machine) => {
      machinesForReservationRecord[machine.name] = {
        machine_id: Number(machine.machine_id),
        platform: machine.platform,
      }
    })

    const res = await safeFetch(
      `${API_URL}/contexts`,
      {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ machines: machinesForReservationRecord }),
      },
      ContextReservationSchema,
    )

    return res
  }

  async function deleteContext(contextId: string): Promise<FetchResult<number>> {
    try {
      const res = await fetch(`${API_URL}/contexts/${contextId}`, {
        method: 'DELETE',
      })
      if (res.status >= 400) {
        return {
          data: null,
          error: `HTTP Status: ${res.status}`,
        }
      }
      return {
        data: res.status,
        error: null,
      }
    } catch (err) {
      return {
        data: null,
        error: String(err),
      }
    }
  }

  async function getLifetimeFromContext(
    contextId: string,
  ): Promise<FetchResult<{ lifetime: number }>> {
    const res = await safeFetch(`${API_URL}/contexts/${contextId}`, {}, ContextLifetimeSchema)
    return res
  }

  async function getMachinesFromContext(contextId: string): Promise<FetchResult<LocalMachine[]>> {
    try {
      const { data, error } = await safeFetch(
        `${API_URL}/contexts/${contextId}/machines`,
        {},
        ContextMachinesSchema,
      )

      if (error) {
        return {
          data: null,
          error: error,
        }
      }

      if (!data) {
        return {
          data: [],
          error: null,
        }
      }

      const machines = await Promise.all(
        data.map(async (x) => {
          const { data: machine_data, error: machine_error } = await getMachine(contextId, x)
          if (machine_data) {
            return {
              name: x,
              machine_id: String(machine_data.id),
              platform: machine_data.platform,
              powerState: 'unknown',
            }
          } else {
            return {
              data: null,
              error: `Could not fetch machine metadata: ${String(machine_error)}`,
            }
          }
        }),
      )

      return {
        data: machines as LocalMachine[],
        error: null,
      }
    } catch (err) {
      return {
        data: null,
        error: String(err),
      }
    }
  }

  async function getMachine(
    contextId: string,
    machineId: string,
  ): Promise<FetchResult<{ id: number; platform: string }>> {
    const res = await safeFetch(
      `${API_URL}/contexts/${contextId}/machines/${machineId}`,
      {},
      ContextMachineSchema,
    )
    return res
  }

  async function getContextFromId(contextId: string): Promise<
    FetchResult<{
      id: string
      lifetime: number
      machines: LocalMachine[]
      networks: {
        name: string
        machines: ApiNetwork
      }[]
    }>
  > {
    // TODO: This is a bit of a request bomb, it would be nice to have an endpoint that does all of this
    const [lifetime, machines, networks] = await Promise.all([
      getLifetimeFromContext(contextId),
      getMachinesFromContext(contextId),
      getAllNetworks(contextId),
    ])

    if (networks.error && machines.error && lifetime.error) {
      return {
        data: null,
        error:
          'Lifetime, machines, and networks were unable to be fetched. Perhaps the context is down?',
      }
    }

    if (networks.error) {
      console.error(`Something went wrong with networks ${networks.error}`)
      return { data: null, error: 'Unable to fetch networks' }
    }

    if (machines.error) {
      console.error(`Something went wrong with machines ${machines.error}`)
      return { data: null, error: 'Unable to fetch machine data' }
    }

    if (lifetime.error || !lifetime.data) {
      console.error(`Something went wrong with lifetimes ${lifetime.error}`)
      return { data: null, error: 'Unable to fetch lifetime data' }
    }

    return {
      data: {
        id: contextId,
        lifetime: lifetime.data.lifetime,
        machines: (machines.data as LocalMachine[]) || [],
        networks: networks.data || [],
      },
      error: null,
    }
  }

  async function setContextLifetime(
    context: Context,
    lifetime: number,
  ): Promise<FetchResult<number>> {
    try {
      const res = await fetch(`${API_URL}/contexts/${context.id}`, {
        method: 'PATCH',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          lifetime: lifetime,
        }),
      })

      if (res.status >= 400) {
        return {
          data: null,
          error: `HTTP Status: ${res.status}`,
        }
      }
      return {
        data: res.status,
        error: null,
      }
    } catch (err) {
      return {
        data: null,
        error: String(err),
      }
    }
  }

  return {
    reserveContext,
    deleteContext,
    getMachine,
    getMachinesFromContext,
    getContextFromId,
    setContextLifetime,
  }
}
