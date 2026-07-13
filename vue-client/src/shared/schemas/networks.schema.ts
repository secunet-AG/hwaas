// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

import z from 'zod'

export const ListMachineNetworksSchema = z.array(z.string())

// This one is fairly nasty. The API just gives a weird Record<string, Record<string,{}>
export const GetNetworkSchema = z.record(z.string(), z.record(z.string(), z.object()))
