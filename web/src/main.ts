import { createPinia } from 'pinia'
import { createApp } from 'vue'

import App from './App.vue'
import { hostKey } from './host'
import { createBrowserHost } from './host/browser'
import { createAppRouter } from './router'

const app = createApp(App)
app.use(createPinia())
app.use(createAppRouter())
app.provide(hostKey, createBrowserHost())
app.mount('#app')
