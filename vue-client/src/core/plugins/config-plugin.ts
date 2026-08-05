// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

import { computed, inject, onMounted, ref, type App, type ComputedRef, type Ref } from 'vue'
import z from 'zod'

/** The injection token for our application config, used for DI */
export const CONFIG_INJECTION_KEY = Symbol('CONFIG_INJECTION_KEY')

/** A runtime type-checked schema for the application environment */
export const ConfigSchema = z.object({
  API_URL: z.string(),
  MAXIMUM_CONTEXT_LIFETIME_SECONDS: z.number(),
  DOCS_URL: z.string(),
})

/** Our expected application config shape, inferred from the schema */
export type Config = z.infer<typeof ConfigSchema>

/** Our global application config state */
export type ConfigState = {
  config: Ref<Config>
  apiUrl: ComputedRef<String>
}

const CONTACT_MSG = 'please contact your HWaaS instance maintainer'

/**
 * A global config provider, this provides a single source
 * of truth for application environment state.
 *
 * We are using a config.json file, as we are primarily exporting this
 * as a static site, with nix, and we do not want separate packages for
 * each instance.
 *
 * So, we simply host a config.json per instance, with our required environment
 **/
export const configProvider = (): ConfigState => {
  const config = ref()

  const apiUrl = computed(() => config.value.apiUrl)

  onMounted(async () => {
    console.info('loading config...')
    try {
      const res = await fetch('/config.json')
      const parsed = ConfigSchema.safeParse(await res.json())
      if (!parsed.success) {
        console.error(`config.json is not valid, ${CONTACT_MSG}: ${parsed.error}`)
        return
      }
      config.value = parsed.data
    } catch (e) {
      console.error(`Could not load config.json, ${CONTACT_MSG}: ${e}`)
    }
  })

  return { config, apiUrl }
}

export const configPlugin = (app: App) => {
  app.provide<ConfigState>(CONFIG_INJECTION_KEY, configProvider())
}

export const useConfig = () => {
  const injected = inject<ConfigState>(CONFIG_INJECTION_KEY)
  if (!injected) throw new Error('Could not inject the config state!')
  return injected
}
