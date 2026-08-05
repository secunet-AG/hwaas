// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

import './assets/main.css'

import { createApp } from 'vue'
import App from './App.vue'
import router from './router.ts'
import { createPinia } from 'pinia'
import { toolTipPlugin } from './core/plugins/tooltip-plugin.ts'
import { configPlugin } from './core/plugins/config-plugin.ts'

const app = createApp(App)
const pinia = createPinia()

app.use(router)
app.use(pinia)
app.use(configPlugin)
app.use(toolTipPlugin)

app.mount('#app')
