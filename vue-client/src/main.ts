import './assets/main.css'

import { createApp } from 'vue'
import App from './App.vue'
import router from './router.ts'
import { createPinia } from 'pinia'
import { apiUrlPlugin } from './core/plugins/apiUrlPlugin.ts'
import { toolTipPlugin } from './core/plugins/tooltip-plugin.ts'

const app = createApp(App)
const pinia = createPinia()

app.use(router)
app.use(pinia)
app.use(apiUrlPlugin)
app.use(toolTipPlugin)

app.mount('#app')
