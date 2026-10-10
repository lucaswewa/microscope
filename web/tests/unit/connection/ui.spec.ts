import { flushPromises, type VueWrapper } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { useConnectionStore } from '@/connection/store'

import { fakeServer, json } from '../api/fakeServer'
import { mountApp } from '../mountApp'

const system = {
  title: 'MicroscopeSystem',
  properties: {
    hostname: { forms: [{ href: '/api/v1/system/hostname', op: ['readproperty'] }] },
  },
}

const camera = { title: 'SimulatedCamera' }

const control = { down: false }
let wrapper: VueWrapper | undefined

beforeEach(() => {
  control.down = false
  window.localStorage.clear()
  window.history.replaceState(null, '', '/')
  const answer = (body: unknown) => () =>
    control.down ? Promise.reject(new TypeError('Failed to fetch')) : json(body)
  vi.stubGlobal(
    'fetch',
    fakeServer({
      'GET /api/v1/thing_descriptions/': answer({ system, camera }),
      'GET /api/v1/system/hostname': answer('lab-pc'),
    }).fetch,
  )
})

afterEach(() => {
  wrapper?.unmount()
  wrapper = undefined
  vi.unstubAllGlobals()
})

async function open(path: string) {
  const app = await mountApp(path)
  wrapper = app.wrapper
  return { ...app, connection: useConnectionStore() }
}

const indicator = () => wrapper!.get('.connection-indicator')

describe('the connection indicator', () => {
  it('shows the state and the microscope’s name, and leads to the Connect screen', async () => {
    const { connection } = await open('/view')
    expect(indicator().attributes('data-state')).toBe('idle')
    expect(indicator().text()).toBe('Not connected')
    await connection.connect({ kind: 'local' })
    await flushPromises()
    expect(indicator().attributes('data-state')).toBe('connected')
    expect(indicator().text()).toBe('lab-pc')
    expect(indicator().attributes('aria-label')).toMatch(/^lab-pc: This server \(/)
    // The tests' memory history writes `/connect`; the app's hash history, `#/connect`.
    expect(indicator().attributes('href')).toMatch(/^#?\/connect$/)
  })
})

describe('the offline overlay', () => {
  it('covers the page while the microscope can’t be reached, and Retry tries again', async () => {
    const { connection } = await open('/view')
    expect(wrapper!.find('[role="alert"]').exists()).toBe(false)
    control.down = true
    await connection.connect({ kind: 'local' })
    await flushPromises()
    const alert = wrapper!.get('[role="alert"]')
    expect(alert.text()).toContain('Can’t reach the microscope')
    expect(alert.text()).toContain('The microscope could not be reached.')
    control.down = false
    await alert
      .findAll('button')
      .find((button) => button.text() === 'Retry')!
      .trigger('click')
    await flushPromises()
    expect(connection.state).toBe('connected')
    expect(wrapper!.find('[role="alert"]').exists()).toBe(false)
    connection.disconnect()
  })

  it('leaves the Connect screen uncovered, and leads to it', async () => {
    const { connection, router } = await open('/view')
    control.down = true
    await connection.connect({ kind: 'local' })
    await flushPromises()
    const choose = wrapper!
      .findAll('[role="alert"] button')
      .find((b) => b.text() === 'Choose a microscope')
    await choose!.trigger('click')
    await flushPromises()
    expect(router.currentRoute.value.name).toBe('connect')
    expect(wrapper!.find('.offline-overlay').exists()).toBe(false)
    connection.disconnect()
  })
})

describe('the Connect screen', () => {
  it('says what’s wrong with an address', async () => {
    await open('/connect')
    await wrapper!.get('input').setValue('ftp://lab-pc')
    await wrapper!.get('form').trigger('submit')
    expect(wrapper!.get('.form-field__text--error').text()).toContain('must start with http://')
    expect(useConnectionStore().state).toBe('idle')
  })

  it('connects to another microscope, then goes to View; it is remembered', async () => {
    const { router, connection } = await open('/connect')
    await wrapper!.get('input').setValue('lab-pc:5000')
    await wrapper!.get('form').trigger('submit')
    await flushPromises()
    expect(connection.profile).toEqual({ kind: 'remote', origin: 'http://lab-pc:5000' })
    expect(router.currentRoute.value.name).toBe('view')
    await router.push('/connect')
    await flushPromises()
    expect(wrapper!.get('.connect__recent').text()).toContain('lab-pc:5000')
    await wrapper!.get('[aria-label="Forget lab-pc:5000"]').trigger('click')
    expect(wrapper!.find('.connect__recent').exists()).toBe(false)
    connection.disconnect()
  })
})
