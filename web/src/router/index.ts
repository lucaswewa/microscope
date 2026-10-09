import {
  createRouter,
  createWebHashHistory,
  type RouteRecordRaw,
  type RouterHistory,
} from 'vue-router'

import PlaceholderView from '@/views/PlaceholderView.vue'

/** Pages for development only: left out of production builds. */
const devRoutes: RouteRecordRaw[] = import.meta.env.DEV
  ? [
      {
        path: '/dev/tokens',
        name: 'dev-tokens',
        component: () => import('@/views/dev/TokensView.vue'),
      },
    ]
  : []

/**
 * The app's router. Routes live in the URL's hash (`/#/control`), which
 * works however the app is served: by the microscope server, by Vite, or
 * from the Tauri app's files (ADR-0010).
 */
export function createAppRouter(history: RouterHistory = createWebHashHistory()) {
  return createRouter({
    history,
    routes: [{ path: '/', name: 'home', component: PlaceholderView }, ...devRoutes],
  })
}
