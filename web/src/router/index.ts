import type { Component } from 'vue'
import {
  createRouter,
  createWebHashHistory,
  type RouteRecordRaw,
  type RouterHistory,
} from 'vue-router'

import { DESTINATIONS } from '@/app/navigation'
import ConnectView from '@/connection/ConnectView.vue'
import PlaceholderPage from '@/views/PlaceholderPage.vue'
import ViewPage from '@/views/ViewPage.vue'

/** The layout each destination's page will have, which its placeholder shows. */
const LAYOUTS: Record<string, { pane?: 'narrow' | 'wide'; liveImage?: boolean }> = {
  control: { pane: 'narrow', liveImage: true },
  'slide-scan': { pane: 'wide', liveImage: true },
  sequence: { pane: 'wide', liveImage: true },
  settings: { pane: 'narrow' },
}

/** The pages built so far; the other destinations show a placeholder. */
const PAGES: Record<string, Component> = { view: ViewPage }

/** One route per destination in the rail. Settings has sections: `/#/settings/camera`. */
const destinationRoutes: RouteRecordRaw[] = DESTINATIONS.map((destination) => ({
  path: destination.id === 'settings' ? '/settings/:section?' : `/${destination.id}`,
  name: destination.id,
  ...(PAGES[destination.id]
    ? { component: PAGES[destination.id] }
    : {
        component: PlaceholderPage,
        props: { destinationId: destination.id, ...LAYOUTS[destination.id] },
      }),
}))

/** Pages for development only: left out of production builds. */
const devRoutes: RouteRecordRaw[] = import.meta.env.DEV
  ? [
      {
        path: '/dev/tokens',
        name: 'dev-tokens',
        component: () => import('@/views/dev/TokensView.vue'),
      },
      {
        path: '/dev/components',
        name: 'dev-components',
        component: () => import('@/views/dev/ComponentsView.vue'),
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
    routes: [
      { path: '/', redirect: { name: 'view' } },
      ...destinationRoutes,
      { path: '/connect', name: 'connect', component: ConnectView },
      ...devRoutes,
      { path: '/:unknown(.*)*', redirect: { name: 'view' } },
    ],
  })
}
