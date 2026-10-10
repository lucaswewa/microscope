import { flushPromises, type VueWrapper } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { ThingDescription } from '@/api/wot/td'
import { useConnectionStore } from '@/connection/store'

import { eventStream, fakeServer, json, type Received } from '../api/fakeServer'
import { mountApp } from '../mountApp'

const form = (name: string, op: string[], sse = false) => ({
  href: `/api/v1/stage/${name}`,
  op,
  ...(sse ? { subprotocol: 'sse' } : {}),
})
const action = (name: string) => ({ forms: [form(name, ['invokeaction'])] })

const stage: ThingDescription = {
  title: 'SimulatedStage',
  base: 'http://localhost/',
  properties: {
    position: {
      forms: [form('position', ['readproperty']), form('position', ['observeproperty'], true)],
    },
    moving: { forms: [form('moving', ['observeproperty'], true)] },
  },
  actions: Object.fromEntries(
    ['move_absolute', 'move_to_origin', 'set_zero_position', 'jog'].map((name) => [
      name,
      action(name),
    ]),
  ),
}
const system = {
  title: 'MicroscopeSystem',
  properties: { hostname: { forms: [{ href: '/api/v1/system/hostname', op: ['readproperty'] }] } },
}

let wrapper: VueWrapper | undefined
let received: Received[]
/** The status of the next invocation: `running` until `finish()`. */
let finished: boolean

/** An invocation record, running or completed. */
const record = (id: string, status: string) => ({
  id,
  status,
  href: `http://localhost/api/v1/action_invocations/${id}`,
  output: null,
})

beforeEach(() => {
  vi.useFakeTimers()
  window.localStorage.clear()
  finished = false
  const server = fakeServer({
    'GET /api/v1/thing_descriptions/': { system, camera: { title: 'Camera' }, stage },
    'GET /api/v1/system/hostname': 'lab-pc',
    'GET /api/v1/stage/position': (request: Received) =>
      request.headers.get('Accept') === 'text/event-stream'
        ? eventStream(['data: {"x": 1, "y": 2, "z": 3}\n\n'], request, { open: true })
        : json({ x: 5, y: 6, z: 7 }),
    'GET /api/v1/stage/moving': (request: Received) => eventStream([], request, { open: true }),
    'POST /api/v1/stage/move_absolute': () => json(record('m', 'running'), 201),
    'GET /api/v1/action_invocations/m': () => json(record('m', finished ? 'completed' : 'running')),
    'DELETE /api/v1/action_invocations/m': () => {
      finished = true
      return json(null)
    },
    'POST /api/v1/stage/move_to_origin': () => json(record('h', 'completed'), 201),
    'POST /api/v1/stage/set_zero_position': () => json(record('z', 'completed'), 201),
    'POST /api/v1/stage/jog': () => json(record('j', 'completed'), 201),
  })
  received = server.received
  vi.stubGlobal('fetch', server.fetch)
})

afterEach(() => {
  wrapper?.unmount()
  wrapper = undefined
  vi.unstubAllGlobals()
  vi.useRealTimers()
})

async function open() {
  wrapper = (await mountApp('/control')).wrapper
  await useConnectionStore().connect({ kind: 'local' })
  await vi.advanceTimersByTimeAsync(0)
  await flushPromises()
}

/** The JSON bodies POSTed to the stage's action `name`. */
const posted = (name: string) =>
  received
    .filter((r) => r.method === 'POST' && r.url.pathname === `/api/v1/stage/${name}`)
    .map((r) => JSON.parse(r.body ?? 'null'))

const button = (name: string) =>
  wrapper!.findAll('button').find((b) => (b.attributes('aria-label') ?? b.text()) === name)!

describe('the Control tab', () => {
  it('shows the live position, and moves to typed coordinates', async () => {
    await open()
    expect(wrapper!.find('.position-section__live').text()).toContain('x 1, y 2, z 3')
    const fields = wrapper!.findAll('.position-section input')
    expect(fields.map((field) => (field.element as HTMLInputElement).value)).toEqual([
      '5',
      '6',
      '7',
    ])

    await fields[0]!.setValue('100')
    await fields[0]!.trigger('keydown', { key: 'Enter' })
    await flushPromises()
    expect(posted('move_absolute')).toEqual([{ x: 100, y: 6, z: 7 }])
    // While it runs, Move becomes Cancel, which cancels the invocation.
    await button('Cancel').trigger('click')
    await vi.advanceTimersByTimeAsync(1000)
    expect(received.some((r) => r.method === 'DELETE')).toBe(true)
    expect(button('Move').exists()).toBe(true)
  })

  it('sets Home, and moves Home only once you confirm', async () => {
    await open()
    await button('Set Home').trigger('click')
    await flushPromises()
    expect(posted('set_zero_position')).toEqual([{}])

    for (const choice of ['Cancel', 'Move Home']) {
      await button('Move Home').trigger('click')
      await flushPromises()
      const dialog = document.querySelector('[role="alertdialog"], [role="dialog"]')!
      expect(dialog.textContent).toContain('Remove your sample first')
      const buttons = [...dialog.querySelectorAll('button')]
      buttons.find((b) => b.textContent?.trim() === choice)!.click()
      await flushPromises()
    }
    expect(posted('move_to_origin')).toEqual([{}])
  })

  it('jogs with the keys while they are held, and with the d-pad', async () => {
    await open()
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowUp', bubbles: true }))
    await vi.advanceTimersByTimeAsync(650)
    window.dispatchEvent(new KeyboardEvent('keyup', { key: 'ArrowUp' }))
    await flushPromises()
    expect(posted('jog')).toEqual([
      { x: 0, y: -200, z: 0 },
      { x: 0, y: -1000, z: 0 },
      { x: 0, y: -1000, z: 0 },
      { stop: true },
    ])

    // A key held as the window loses focus never sends its key-up: stop anyway.
    received.length = 0
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'PageUp', bubbles: true }))
    await vi.advanceTimersByTimeAsync(350)
    window.dispatchEvent(new Event('blur'))
    await flushPromises()
    expect(posted('jog').at(-1)).toEqual({ stop: true })

    received.length = 0
    const right = button('Move right')
    await right.trigger('pointerdown', { button: 0, pointerId: 1 })
    await right.trigger('pointerup', { pointerId: 1 })
    await right.trigger('click', { detail: 0 }) // Enter, from the keyboard
    await flushPromises()
    expect(posted('jog')).toEqual([
      { x: 200, y: 0, z: 0 },
      { x: 200, y: 0, z: 0 },
    ])
  })
})
