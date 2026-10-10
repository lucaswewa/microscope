import { mount, type VueWrapper } from '@vue/test-utils'
import { afterEach, describe, expect, it } from 'vitest'
import { defineComponent } from 'vue'

import { DESTINATIONS } from '@/app/navigation'
import { useHost } from '@/host'

import { mountApp } from './mountApp'

describe('the app', () => {
  let wrapper: VueWrapper | undefined

  afterEach(() => {
    wrapper?.unmount()
    wrapper = undefined
  })

  it('opens on the View page', async () => {
    const app = await mountApp('/')
    wrapper = app.wrapper
    expect(app.router.currentRoute.value.name).toBe('view')
    expect(wrapper.get('h1').text()).toBe('View')
  })

  it('sends unknown addresses to the View page', async () => {
    const app = await mountApp('/no/such/page')
    wrapper = app.wrapper
    expect(app.router.currentRoute.value.name).toBe('view')
  })

  it('shows the live image on the View page', async () => {
    const app = await mountApp('/view')
    wrapper = app.wrapper
    expect(wrapper.get('h1').text()).toBe('View')
    expect(wrapper.find('.live-image').exists()).toBe(true)
  })

  it('shows the stage controls beside the live image on the Control page', async () => {
    const app = await mountApp('/control')
    wrapper = app.wrapper
    expect(wrapper.get('h1').text()).toBe('Control')
    expect(wrapper.find('.direction-pad').exists()).toBe(true)
    expect(wrapper.find('.live-image').exists()).toBe(true)
  })

  for (const destination of DESTINATIONS.filter(({ id }) => !['view', 'control'].includes(id))) {
    it(`has a placeholder for ${destination.label}, built in ${destination.builtIn}`, async () => {
      const app = await mountApp(`/${destination.id}`)
      wrapper = app.wrapper
      expect(wrapper.get('h1').text()).toBe(destination.label)
      expect(wrapper.text()).toContain(`built in ${destination.builtIn}`)
    })
  }

  it('shows the Settings section from the address', async () => {
    const app = await mountApp('/settings/camera')
    wrapper = app.wrapper
    expect(app.router.currentRoute.value.name).toBe('settings')
    expect(wrapper.text()).toContain('Section: camera')
  })

  it('has a gallery of the UI components in both themes, in development', async () => {
    const app = await mountApp('/dev/components')
    wrapper = app.wrapper
    expect(app.router.currentRoute.value.name).toBe('dev-components')
    const panels = wrapper.findAll('section[data-theme]')
    expect(panels.map((panel) => panel.attributes('data-theme'))).toEqual(['light', 'dark'])
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
