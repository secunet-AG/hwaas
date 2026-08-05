// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

import { inject, type App } from 'vue'
import z from 'zod'

export const CONFIG_INJECTION_KEY = Symbol('CONFIG_INJECTION_KEY')

export const ConfigSchema = z.object({
  API_URL: z.string(),
  MAXIMUM_CONTEXT_LIFETIME_SECONDS: z.coerce.number(),
  DOCS_URL: z.string(),
})

export type Config = z.infer<typeof ConfigSchema>

const CONTACT_MSG = 'please contact your HWaaS instance maintainer'

/**
 * Loads and validates config once, before mount. Throws on failure so
 * the app never starts in a half-configured state.
 *
 * Static-site deploy (nix), so production reads a per-instance /config.json
 * rather than baked-in env.
 */
export async function loadConfig(): Promise<Config> {
  if (!import.meta.env.PROD) {
    return ConfigSchema.parse({
      API_URL: import.meta.env.VITE_API_URL,
      MAXIMUM_CONTEXT_LIFETIME_SECONDS: import.meta.env.VITE_MAXIMUM_CONTEXT_LIFETIME_SECONDS,
      DOCS_URL: import.meta.env.VITE_DOCS_URL,
    })
  }

  let json: unknown
  try {
    json = await (await fetch('/config.json')).json()
  } catch (e) {
    throw new Error(`Could not load config.json, ${CONTACT_MSG}: ${e}`)
  }

  const parsed = ConfigSchema.safeParse(json)
  if (!parsed.success) {
    throw new Error(`config.json is not valid, ${CONTACT_MSG}: ${parsed.error}`)
  }
  return parsed.data
}

export function provideConfig(app: App, config: Config) {
  app.provide(CONFIG_INJECTION_KEY, config)
}

export function useConfig(): Config {
  const config = inject<Config>(CONFIG_INJECTION_KEY)
  if (!config) throw new Error('Config was not provided!')
  return config
}
