// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

import type { Component } from 'vue'
import { GlobeIcon, CloudIcon, ChipIcon, CubeIcon, SettingsIcon } from '../../icons'

export interface SidebarRoute {
  name: string
  displayName: string
  icon: Component
}

export const SIDEBAR_CONFIG = [
  {
    name: 'home',
    displayName: 'Home',
    icon: GlobeIcon,
  },
  {
    name: 'contexts',
    displayName: 'Contexts',
    icon: CloudIcon,
  },
  // {
  //   name: 'machines',
  //   displayName: 'Machines',
  //   icon: ChipIcon,
  // },
  {
    name: 'images',
    displayName: 'Images',
    icon: CubeIcon,
  },
  {
    name: 'settings',
    displayName: 'Settings',
    icon: SettingsIcon,
  },
] satisfies SidebarRoute[]
