// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

import './assets/main.css'
import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import router from './router.ts'
import { toolTipPlugin } from './core/plugins/tooltip-plugin.ts'
import { loadConfig, provideConfig } from './core/plugins/config-plugin.ts'

const config = await loadConfig()

const app = createApp(App)
provideConfig(app, config)
app.use(router)
app.use(createPinia())
app.use(toolTipPlugin)
app.mount('#app')
