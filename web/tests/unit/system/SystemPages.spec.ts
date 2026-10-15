import { flushPromises, type VueWrapper } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { Router } from 'vue-router'

import type { ThingDescription } from '@/api/wot/td'
import { useConnectionStore } from '@/connection/store'
import { hostKey } from '@/host'
import { createBrowserHost } from '@/host/browser'

import { fakeServer, json, type Received } from '../api/fakeServer'
import { mountApp } from '../mountApp'

/** A Thing's description: its type as its title, with readable properties and actions. */
function thing(title: string, name: string, properties: string[], actions: string[] = []) {
  const href = (affordance: string) => `/api/v1/${name}/${affordance}`
  return {
    title,
    base: 'http://localhost/',
    properties: Object.fromEntries(
      properties.map((p) => [p, { forms: [{ href: href(p), op: ['readproperty'] }] }]),
    ),
    actions: Object.fromEntries(
      actions.map((a) => [a, { forms: [{ href: href(a), op: ['invokeaction'] }] }]),
    ),
  } as ThingDescription
}

const done = (id: string) =>
  json({ id, status: 'completed', href: `http://localhost/api/v1/action_invocations/${id}` }, 201)

let managed: boolean
let shutdown: () => Response
let received: Received[]
let wrapper: VueWrapper | undefined
let router: Router
const openLink = vi.fn()

beforeEach(() => {
  vi.useFakeTimers()
  managed = false
  shutdown = () => done('s')
  const server = fakeServer({
    'GET /api/v1/thing_descriptions/': {
      system: thing(
        'MicroscopeSystem',
        'system',
        ['hostname', 'version_data', 'os_version', 'managed'],
        ['shutdown', 'restart'],
      ),
      camera: thing('SimulatedCamera', 'camera', []),
      illumination: thing('SimulatedIllumination', 'illumination', [], ['flash']),
    },
    'GET /api/v1/system/hostname': 'lab-pc',
    'GET /api/v1/system/version_data': { version: '0.1.0', version_source: 'v0.1.0-46-gabc1234' },
    'GET /api/v1/system/os_version': 'Windows 11 Pro',
    'GET /api/v1/system/managed': () => json(managed),
    'POST /api/v1/system/shutdown': () => shutdown(),
    'POST /api/v1/system/restart': () => done('r'),
    'POST /api/v1/illumination/flash': () => done('f'),
    'GET /api/v1/action_invocations/f': () => json({ id: 'f', status: 'completed' }),
    'GET /api/v1/health': { status: 'ok' },
  })
  received = server.received
  vi.stubGlobal('fetch', server.fetch)
})

afterEach(() => {
  wrapper?.unmount()
  wrapper = undefined
  document.body.innerHTML = ''
  vi.clearAllMocks()
  vi.unstubAllGlobals()
  vi.useRealTimers()
})

async function open(path: string, remote = false) {
  const host = { ...createBrowserHost(), openLink }
  ;({ wrapper, router } = await mountApp(path, { [hostKey]: host }))
  const profile = remote
    ? { kind: 'remote' as const, origin: 'http://lab-pc:5000' }
    : { kind: 'local' as const }
  await useConnectionStore().connect(profile)
  await vi.advanceTimersByTimeAsync(10)
}

const posted = (path: string) =>
  received.filter((r) => r.method === 'POST' && r.url.pathname === `/api/v1/${path}`)
const button = (label: string) => wrapper!.findAll('button').find((b) => b.text() === label)
/** The pairs of a page's lists of facts. */
const facts = () =>
  Object.fromEntries(
    wrapper!.findAll('dt').map((dt) => [dt.text(), dt.element.nextElementSibling!.textContent]),
  )
/** Answers the open question with its button `label`. */
async function answer(label: string) {
  const question = document.querySelector<HTMLElement>('[role="alertdialog"]')!
  ;[...question.querySelectorAll('button')].find((b) => b.textContent!.trim() === label)!.click()
  await flushPromises()
}

describe('the About tab', () => {
  it('says what the microscope is, finds it with a flash, and links to the project', async () => {
    managed = true
    await open('/about')
    expect(facts()).toMatchObject({
      Computer: 'lab-pc',
      Address: 'localhost:3000',
      'Server version': '0.1.0 (v0.1.0-46-gabc1234)',
      'Operating system': 'Windows 11 Pro',
      'Managed by the launcher': 'Yes: it can restart',
      Camera: 'SimulatedCamera',
      Stage: 'None',
      Illumination: 'SimulatedIllumination',
    })
    expect(wrapper!.text()).toContain('It isn’t a medical device')

    await button('Flash illumination')!.trigger('click')
    await flushPromises()
    expect(posted('illumination/flash')).toHaveLength(1)
    await wrapper!.get('a[href$="/issues"]').trigger('click')
    expect(openLink).toHaveBeenCalledWith('https://github.com/lucaswewa/microscope/issues')
  })
})

describe('the Power tab', () => {
  it('shuts the server down after asking, then offers to reconnect', async () => {
    await open('/power')
    expect([button('Restart'), button('Disconnect')]).toEqual([undefined, undefined])
    await button('Shut down')!.trigger('click')
    await answer('Cancel')
    expect(posted('system/shutdown')).toHaveLength(0)

    await button('Shut down')!.trigger('click')
    await answer('Shut down')
    expect(posted('system/shutdown')).toHaveLength(1)
    const connection = useConnectionStore()
    expect([connection.state, connection.shutDown]).toEqual(['idle', true])
    expect(wrapper!.text()).toContain('The microscope has shut down')
    // It doesn't try to reach the server meanwhile.
    const asked = received.length
    await vi.advanceTimersByTimeAsync(30_000)
    expect(received.length).toBe(asked)

    await button('Reconnect')!.trigger('click')
    await flushPromises()
    expect([connection.state, connection.shutDown]).toEqual(['connected', false])
    expect(wrapper!.text()).not.toContain('has shut down')
  })

  it('says when the server couldn’t be shut down', async () => {
    shutdown = () => json({ detail: 'busy' }, 500)
    await open('/power')
    await button('Shut down')!.trigger('click')
    await answer('Shut down')
    expect(document.body.textContent).toContain('The server couldn’t be shut down.')
    expect(useConnectionStore().shutDown).toBe(false)
  })

  it('restarts a managed server after asking', async () => {
    managed = true
    await open('/power')
    await button('Restart')!.trigger('click')
    await answer('Restart')
    expect(posted('system/restart')).toHaveLength(1)
    expect(document.body.textContent).toContain('The server is restarting.')
  })

  it('disconnects from another microscope', async () => {
    await open('/power', true)
    await button('Disconnect')!.trigger('click')
    await flushPromises()
    expect(useConnectionStore().state).toBe('idle')
    expect(router.currentRoute.value.name).toBe('connect')
  })
})
