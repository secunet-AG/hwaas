// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

// This type is currently used when receiving and sending machines to the API

import type { LocalImage } from './images.model'

// Locally, we use a different shape to more easily interact with it
export type ContextConfigurationMachine = Record<
  string,
  { machine_id: number | string; platform: string }
>

export type InventoryMachineState = 'free' | 'registered'

export interface InventoryMachine {
  machine_id: number
  estimated_reservation_duration: number
  state: InventoryMachineState
  properties: {
    platform: string
  }
}

export type MachineWithPort = LocalMachine & {
  isSelected: boolean
  ports?: string[]
  selectedPort?: string
}

type ActiveDrive = string
export type PowerState = 'on' | 'off' | 'unknown'

// This interface is used to more easily interact with machines than the above record shape
export interface LocalMachine {
  name: string
  machine_id: string | number
  platform: string
  activeImage?: LocalImage
  serialPorts: string[]
  powerState: PowerState
  activeDrive?: ActiveDrive
}

export type MachineName = string
export type NetworkPort = Record<string, void>
export type Network = { name: string; machines?: Record<MachineName, NetworkPort> }

export interface LocalNetwork {
  name: string
  networkMachines: {
    name: string
    port: string
  }[]
}

export interface Context {
  id: string
  name: string // Not currently persisted
  lifetime: number // Remaining seconds for the context
  machines: LocalMachine[]
  networks: Network[]
}
