import { createRouter, createWebHashHistory, type RouterHistory } from 'vue-router'

import PlaceholderView from '@/views/PlaceholderView.vue'

/**
 * The app's router. Routes live in the URL's hash (`/#/control`), which
 * works however the app is served: by the microscope server, by Vite, or
 * from the Tauri app's files (ADR-0010).
 */
export function createAppRouter(history: RouterHistory = createWebHashHistory()) {
  return createRouter({
    history,
    routes: [{ path: '/', name: 'home', component: PlaceholderView }],
  })
}
