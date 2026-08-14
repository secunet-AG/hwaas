// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

import { useConfig } from '@/core/plugins/config-plugin'
import type { FetchResult } from '../safeFetch'

export const useDrivesApi = () => {
  const { API_URL } = useConfig()

  async function createDrive(
    contextId: string,
    driveId: string,
    imageHash: string,
  ): Promise<FetchResult<number>> {
    try {
      const res = await fetch(
        `${API_URL}/contexts/${contextId}/drives/${driveId}?image_hash=${imageHash}`,
        {
          method: 'PUT',
        },
      )
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
    createDrive,
  }
}
