import { useApiUrl } from '@/core/plugins/apiUrlPlugin'
import { safeFetch, type FetchResult } from '../safeFetch'
import { GetNetworkSchema, ListMachineNetworksSchema } from '@/shared/schemas/networks.schema'
import type { ApiNetwork } from '@/shared/types/networks.model'

export interface NetworkPatchPath {
  path: string
}

export interface NetworkAddOp {
  op: 'add'
  value: any
}

export interface NetworkRemoveOp {
  op: 'remove'
}

export type NetworkPathProps = (NetworkPatchPath & NetworkAddOp) | NetworkRemoveOp

export const useNetworksApi = () => {
  const { apiUrl } = useApiUrl()

  async function listMachineNetworks(contextId: string): Promise<FetchResult<string[]>> {
    const res = await safeFetch<string[]>(
      `${apiUrl.value}/contexts/${contextId}/networks`,
      {
        headers: { Accept: 'application/json' },
      },
      ListMachineNetworksSchema,
    )
    return res
  }

  async function getAllNetworksInContext(
    contextId: string,
  ): Promise<FetchResult<{ name: string; machines: ApiNetwork }[]>> {
    const allNetworkNames = await listMachineNetworks(contextId)

    if (allNetworkNames.error || !allNetworkNames.data) {
      return {
        data: null,
        error: `Something went wrong with fetching networks for context ${contextId}`,
      }
    }

    const allNetworkRes = await Promise.all(
      allNetworkNames.data.map(async (x) => [x, await getNetwork(contextId, x)] as const),
    )

    const allNetworksInContext = allNetworkRes
      .filter((x) => !x[1].error && x[1].data)
      .map(([x, res]) => ({ name: x, machines: res.data as ApiNetwork }))

    return {
      data: allNetworksInContext,
      error: null,
    }
  }

  async function deleteNetwork(contextId: string, network: string): Promise<FetchResult<number>> {
    try {
      const res = await fetch(`${apiUrl.value}/contexts/${contextId}/networks/${network}`, {
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

  async function getNetwork(contextId: string, network: string): Promise<FetchResult<ApiNetwork>> {
    const res = await safeFetch(
      `${apiUrl.value}/contexts/${contextId}/networks/${network}`,
      {
        headers: {
          Accept: 'application/json',
        },
      },
      GetNetworkSchema,
    )

    return res
  }

  // TODO: A post request with body would be more semantic imo
  async function createNetwork(
    contextId: string,
    network: string,
    payload: Record<string, Record<string, any>> = {},
  ): Promise<FetchResult<number>> {
    try {
      const res = await fetch(`${apiUrl.value}/contexts/${contextId}/networks/${network}`, {
        method: 'PUT',
        body: JSON.stringify(payload),
        headers: {
          'Content-Type': 'application/json',
        },
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

  async function addMachinesToNetwork(
    contextId: string,
    network: string,
    payload: {
      machineName: string
      port: string
    }[],
  ): Promise<FetchResult<number>> {
    // Create op for JSON body.
    const body = payload.map((x) => ({
      op: 'add',
      path: `/${x.machineName}`,
      value: { [x.port]: {} },
    }))
    try {
      const res = await fetch(`${apiUrl.value}/contexts/${contextId}/networks/${network}`, {
        method: 'PATCH',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(body),
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

  async function deleteMachineFromNetworkByName(
    contextId: string,
    network: string,
    payload: {
      name: string
      port: string
    },
  ): Promise<FetchResult<number>> {
    try {
      const res = await fetch(`${apiUrl.value}/contexts/${contextId}/networks/${network}`, {
        method: 'PATCH',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify([
          {
            op: 'remove',
            path: `/${payload.name}`,
            value: { [payload.port]: {} },
          },
        ]),
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
    } catch (error) {
      return { data: null, error: String(error) }
    }
  }

  async function patchNetwork(
    contextId: string,
    network: string,
    props: NetworkPathProps[],
  ): Promise<FetchResult<number>> {
    try {
      const res = await fetch(`${apiUrl.value}/contexts/${contextId}/networks/${network}`, {
        method: 'PATCH',
        body: JSON.stringify(props),
        headers: {
          'Content-Type': 'application/json',
        },
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
    } catch (error) {
      return { data: null, error: String(error) }
    }
  }

  return {
    listNetworks: listMachineNetworks,
    deleteNetwork,
    getNetwork,
    getAllNetworks: getAllNetworksInContext,
    createNetwork,
    patchNetwork,
    addMachinesToNetwork,
    deleteMachineFromNetworkByName,
  }
}
