// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

import type { Context, LocalMachine, PowerState } from '@/shared/types/contexts.model'
import { useConfig } from '@/core/plugins/config-plugin'

import {
  MachinePowerResponseSchema,
  NetworkInterfacesSchema,
  SerialResponseSchema,
} from '@/shared/schemas/machines.schema'
import { flattenResultArray, safeFetch, type FetchResult } from '../safeFetch'

export type MachinePowerState = {
  power_id: string
  state: boolean
}

export const useMachinesApi = () => {
  const { apiUrl } = useConfig()

  async function powerOnMachine(
    contextId: string,
    machineName: string,
  ): Promise<FetchResult<MachinePowerState[]>> {
    const res = await safeFetch<MachinePowerState[]>(
      `${apiUrl.value}/contexts/${contextId}/machines/${machineName}/power`,
      {
        method: 'PUT',
        headers: {
          Accept: 'application/json',
        },
      },
      MachinePowerResponseSchema,
    )
    return res
  }

  async function powerOffMachine(
    contextId: string,
    machineName: string,
  ): Promise<FetchResult<MachinePowerState[]>> {
    const res = await safeFetch(
      `${apiUrl.value}/contexts/${contextId}/machines/${machineName}/power`,
      {
        method: 'DELETE',
        headers: {
          Accept: 'text/plain',
        },
      },
      MachinePowerResponseSchema,
    )
    return res
  }

  async function getMachinePower(
    contextId: string,
    machineName: string,
  ): Promise<FetchResult<MachinePowerState[]>> {
    const res = await safeFetch(
      `${apiUrl.value}/contexts/${contextId}/machines/${machineName}/power`,
      {
        headers: {
          Accept: 'application/json',
        },
      },
      MachinePowerResponseSchema,
    )
    return res
  }

  async function getAllMachinesPower(
    contextId: string,
    machines: LocalMachine[],
  ): Promise<FetchResult<LocalMachine[]>> {
    try {
      const results = await Promise.allSettled(
        machines.map(async (x) => {
          const { data, error } = await getMachinePower(contextId, x.name)
          const state = data?.find((x) => !!x.state)
          if (error || !data) {
            return { ...x, powerState: 'unknown' as PowerState }
          }
          return { ...x, powerState: (state ? 'on' : 'off') as PowerState }
        }),
      )
      const validMachines = results.filter((x) => x.status === 'fulfilled').map((x) => x.value)
      return {
        data: validMachines,
        error: null,
      }
    } catch (err) {
      return {
        data: null,
        error: String(err),
      }
    }
  }

  async function enableUsb(
    contextId: string,
    machineName: string,
    driveName: string,
  ): Promise<FetchResult<string>> {
    try {
      const res = await fetch(`${apiUrl.value}/contexts/${contextId}/machines/${machineName}/usb`, {
        body: JSON.stringify([
          {
            type: 'storage',
            luns: [
              {
                path: driveName,
                cdrom: false,
                read_only: false,
              },
            ],
          },
          {
            type: 'keyboard',
          },
          {
            type: 'mouse',
          },
        ]),
        method: 'PUT',
        headers: {
          'Content-Type': 'application/json',
          Accept: 'application/string',
        },
      })
      return { data: String(res.status), error: null }
    } catch (err) {
      return {
        data: null,
        error: String(err),
      }
    }
  }

  async function getAllMachinesSerial(
    contextId: string,
    machines: LocalMachine[],
  ): Promise<FetchResult<LocalMachine[]>> {
    const promises = machines.map(async (x) => [x, await getSerial(contextId, x.name)] as const)

    const resArr = await Promise.all(promises)

    // Reduce promises, warn the console if any of our machine serial ports were not found.
    const filtered = resArr.reduce(
      (acc, [x, { data }]) =>
        data
          ? [
              ...acc,
              {
                ...x,
                serialPorts: data,
              },
            ]
          : (() => {
              console.warn(`Could not fetch serial port for machine ${x.name}`)
              return acc
            })(),
      [] as LocalMachine[],
    )

    return {
      data: filtered,
      error: null,
    }
  }

  async function getSerial(contextId: string, machineName: string): Promise<FetchResult<string[]>> {
    const res = await safeFetch(
      `${apiUrl.value}/contexts/${contextId}/machines/${machineName}/serial`,
      {
        headers: {
          Accept: 'application/json',
        },
      },
      SerialResponseSchema,
    )

    return res
  }

  async function getNetworkInterfaces(
    contextId: string,
    machineName: string,
  ): Promise<FetchResult<string[]>> {
    const res = await safeFetch(
      `${apiUrl.value}/contexts/${contextId}/machines/${machineName}/network-interfaces`,
      {
        headers: {
          Accept: 'application/json',
        },
      },
      NetworkInterfacesSchema,
    )

    return res
  }

  async function invalidateMachine(
    contextId: string,
    machine: LocalMachine,
  ): Promise<FetchResult<LocalMachine>> {
    const [powerResponse, serialResponse] = await Promise.all([
      getMachinePower(contextId, machine.name),
      getSerial(contextId, machine.name),
    ])

    if (powerResponse.error) {
      console.error(powerResponse.error)
      return {
        data: null,
        error: `Power response error ${powerResponse.error}`,
      }
    }

    if (serialResponse.error) {
      console.error(serialResponse.error)
      return {
        data: null,
        error: `Serial response error ${serialResponse.error}`,
      }
    }

    const powerState = powerResponse.data && powerResponse.data[0]?.state ? 'on' : 'off'

    return {
      data: {
        ...machine,
        serialPorts: serialResponse.data ?? [],
        powerState: powerState, // We also have unknown, so not just using booleans. Enums in TS can also be somewhat off at times so using string literals instead
      },
      error: null,
    }
  }

  async function invalidateMachines(context: Context): Promise<FetchResult<LocalMachine[]>> {
    const res = await Promise.all(context.machines.map((x) => invalidateMachine(context.id, x)))
    const flat = flattenResultArray(res)
    return flat
  }

  return {
    powerOnMachine,
    powerOffMachine,
    getMachinePower,
    getSerial,
    getNetworkInterfaces,
    getAllMachinesPower,
    getAllMachinesSerial,
    enableUsb,
    invalidateMachines,
  }
}
