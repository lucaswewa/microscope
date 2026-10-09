import { mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { describe, expect, it } from 'vitest'
import { defineComponent } from 'vue'
import { createMemoryHistory } from 'vue-router'

import App from '@/App.vue'
import { hostKey, useHost } from '@/host'
import { createBrowserHost } from '@/host/browser'
import { createAppRouter } from '@/router'

describe('the app', () => {
  it('shows the placeholder page at the root route', async () => {
    const router = createAppRouter(createMemoryHistory())
    await router.push('/')

    const wrapper = mount(App, {
      global: {
        plugins: [createPinia(), router],
        provide: { [hostKey as symbol]: createBrowserHost() },
      },
    })

    expect(wrapper.get('h1').text()).toBe('Microscope')
    expect(wrapper.text()).toContain(`Web app ${__APP_VERSION__}`)
  })

  it('says so when no host is provided', () => {
    const NeedsHost = defineComponent({
      setup() {
        useHost()
        return () => null
      },
    })
    expect(() => mount(NeedsHost)).toThrow('No host is provided')
  })
})
