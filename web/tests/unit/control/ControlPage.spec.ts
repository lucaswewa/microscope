import { flushPromises, type VueWrapper } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { ThingDescription } from '@/api/wot/td'
import { useConnectionStore } from '@/connection/store'
import { hostKey } from '@/host'
import { createBrowserHost } from '@/host/browser'
import { useCaptureFlashes } from '@/live/flash'
import LiveImage from '@/live/LiveImage.vue'

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
const autofocus: ThingDescription = {
  title: 'Autofocus',
  base: 'http://localhost/',
  actions: {
    fast_autofocus: {
      forms: [{ href: '/api/v1/autofocus/fast_autofocus', op: ['invokeaction'] }],
    },
  },
}
const mapping: ThingDescription = {
  title: 'CameraStageMapping',
  base: 'http://localhost/',
  properties: {
    image_resolution: {
      forms: [{ href: '/api/v1/camera_stage_mapping/image_resolution', op: ['readproperty'] }],
    },
  },
  actions: {
    move_in_image_coordinates: {
      forms: [
        { href: '/api/v1/camera_stage_mapping/move_in_image_coordinates', op: ['invokeaction'] },
      ],
    },
  },
}
const camera: ThingDescription = {
  title: 'Camera',
  base: 'http://localhost/',
  actions: {
    capture: { forms: [{ href: '/api/v1/camera/capture', op: ['invokeaction'] }] },
  },
}
const system = {
  title: 'MicroscopeSystem',
  properties: { hostname: { forms: [{ href: '/api/v1/system/hostname', op: ['readproperty'] }] } },
}

let wrapper: VueWrapper | undefined
let received: Received[]
/** The status of the next invocation: `running` until `finish()`. */
let finished: boolean
/** Whether the autofocus invocation has been cancelled. */
let focusCancelled: boolean
/** What autofocus returns, once it has; it runs until then. */
let focusOutput: object | undefined
/** The frames' size when the mapping was calibrated, if it was. */
let resolution: number[] | null
/** The events the stage's `moving` sends. */
let movingEvents: string[]

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
  focusCancelled = false
  focusOutput = undefined
  resolution = [820, 616]
  movingEvents = []
  const server = fakeServer({
    'GET /api/v1/thing_descriptions/': {
      system,
      camera,
      stage,
      autofocus,
      camera_stage_mapping: mapping,
    },
    'GET /api/v1/system/hostname': 'lab-pc',
    'GET /api/v1/stage/position': (request: Received) =>
      request.headers.get('Accept') === 'text/event-stream'
        ? eventStream(['data: {"x": 1, "y": 2, "z": 3}\n\n'], request, { open: true })
        : json({ x: 5, y: 6, z: 7 }),
    'GET /api/v1/stage/moving': (request: Received) =>
      eventStream(movingEvents, request, { open: true }),
    'POST /api/v1/stage/move_absolute': () => json(record('m', 'running'), 201),
    'GET /api/v1/action_invocations/m': () => json(record('m', finished ? 'completed' : 'running')),
    'DELETE /api/v1/action_invocations/m': () => {
      finished = true
      return json(null)
    },
    'POST /api/v1/stage/move_to_origin': () => json(record('h', 'completed'), 201),
    'POST /api/v1/stage/set_zero_position': () => json(record('z', 'completed'), 201),
    'POST /api/v1/stage/jog': () => json(record('j', 'completed'), 201),
    'POST /api/v1/autofocus/fast_autofocus': () => json(record('f', 'running'), 201),
    'GET /api/v1/action_invocations/f': () =>
      json(
        focusOutput
          ? { ...record('f', 'completed'), output: focusOutput }
          : record('f', focusCancelled ? 'cancelled' : 'running'),
      ),
    'DELETE /api/v1/action_invocations/f': () => {
      focusCancelled = true
      return json(null)
    },
    'GET /api/v1/camera_stage_mapping/image_resolution': () => json(resolution),
    'POST /api/v1/camera_stage_mapping/move_in_image_coordinates': () =>
      json(record('c', 'running'), 201),
    'GET /api/v1/action_invocations/c': () => json(record('c', finished ? 'completed' : 'running')),
    'POST /api/v1/camera/capture': () =>
      json(
        { ...record('p', 'completed'), output: { href: 'http://localhost/api/v1/blob/p1' } },
        201,
      ),
    'GET /api/v1/blob/p1': () => new Response('jpeg'),
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

const saveFile = vi.fn(async () => {})

async function open() {
  const host = { ...createBrowserHost(), saveFile }
  wrapper = (await mountApp('/control', { [hostKey]: host })).wrapper
  await useConnectionStore().connect({ kind: 'local' })
  await vi.advanceTimersByTimeAsync(0)
  await flushPromises()
}

/** The JSON bodies POSTed to the stage's action `name`. */
const posted = (name: string) =>
  received
    .filter((r) => r.method === 'POST' && r.url.pathname === `/api/v1/stage/${name}`)
    .map((r) => JSON.parse(r.body ?? 'null'))

const autofocusRuns = () =>
  received
    .filter((r) => r.method === 'POST' && r.url.pathname === '/api/v1/autofocus/fast_autofocus')
    .map((r) => JSON.parse(r.body ?? 'null'))

const captures = () =>
  received
    .filter((r) => r.method === 'POST' && r.url.pathname === '/api/v1/camera/capture')
    .map((r) => JSON.parse(r.body ?? 'null'))

const clickMoves = () =>
  received
    .filter((r) => r.method === 'POST' && r.url.pathname.endsWith('/move_in_image_coordinates'))
    .map((r) => JSON.parse(r.body ?? 'null'))

/** Double-clicks the live image at `point`, as shares of its size from its centre. */
async function doubleClick(point: { x: number; y: number }) {
  wrapper!.findComponent(LiveImage).vm.$emit('doubleClick', point)
  await flushPromises()
}

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

  it('says when the stage is moving, in a line that is always there', async () => {
    await open()
    const status = () => wrapper!.find('.position-section [role="status"]')
    expect(status().exists()).toBe(true)
    expect(status().text()).toBe('')

    wrapper!.unmount()
    movingEvents = ['data: true\n\n']
    await open()
    expect(status().text()).toBe('Moving…')
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

  it('autofocuses from the a key or the button, and cancels', async () => {
    await open()
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'a', bubbles: true }))
    await flushPromises()
    expect(autofocusRuns()).toEqual([{ dz: 2000 }])
    // While it runs, the key doesn't start another, and the button cancels it.
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'a', bubbles: true }))
    await button('Cancel autofocus').trigger('click')
    await vi.advanceTimersByTimeAsync(1000)
    expect(received.some((r) => r.method === 'DELETE' && r.url.pathname.endsWith('/f'))).toBe(true)
    await button('Autofocus').trigger('click')
    await flushPromises()
    expect(autofocusRuns()).toHaveLength(2)
  })

  it('says when there is nothing to focus on', async () => {
    focusOutput = { points: [], focus_z: null, sweeps: 1 }
    await open()
    await button('Autofocus').trigger('click')
    await vi.advanceTimersByTimeAsync(1000)
    const toast = document.querySelector('.overlay-toast__text')
    expect(toast?.textContent).toContain('Nothing to focus on here')
  })

  it('jogs and focuses as the navigation preferences say', async () => {
    window.localStorage.setItem(
      'microscope.navigation',
      JSON.stringify({ steps: { x: 200, y: 30, z: 10 }, invert: { y: true } }),
    )
    await open()
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowUp', bubbles: true }))
    window.dispatchEvent(new KeyboardEvent('keyup', { key: 'ArrowUp' }))
    await wrapper!.get('.live-image').trigger('wheel', { deltaY: -200 })
    await vi.advanceTimersByTimeAsync(100)
    expect(posted('jog')).toEqual([{ x: 0, y: 30, z: 0 }, { z: 20 }])
    // The Navigation section changes them.
    await button('Navigation').trigger('click')
    await wrapper!.get('.navigation-section input[type="checkbox"]').setValue(true)
    expect(JSON.parse(window.localStorage.getItem('microscope.navigation')!).invert.x).toBe(true)
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

  it('moves to a double-clicked point, in the calibration’s pixels, one move at a time', async () => {
    await open()
    await doubleClick({ x: 0.25, y: -0.125 })
    await doubleClick({ x: 0.5, y: 0 }) // ignored: the first is still moving
    expect(clickMoves()).toEqual([{ x: 205, y: -77 }])
    finished = true
    await vi.advanceTimersByTimeAsync(1000)
    await doubleClick({ x: 0.5, y: 0 })
    expect(clickMoves()).toEqual([
      { x: 205, y: -77 },
      { x: 410, y: 0 },
    ])
  })

  it('asks for the mapping to be calibrated before moving by double-click', async () => {
    resolution = null
    await open()
    await doubleClick({ x: 0.25, y: 0 })
    expect(clickMoves()).toEqual([])
    const toast = document.querySelector('.overlay-toast__text')
    expect(toast?.textContent).toContain('Calibrate the camera–stage mapping first')
  })

  it('captures to the gallery or downloads, from the button or the c key', async () => {
    const flashes = useCaptureFlashes()
    await open()
    const before = flashes.value
    await button('Capture').trigger('click')
    await flushPromises()
    expect(captures()).toEqual([{ capture_mode: 'standard', retain_image: true }])
    expect(flashes.value).toBe(before + 1)

    // Download instead: the choice is kept, and the file saved.
    await wrapper!.get('.capture-control [role="combobox"]').trigger('keydown', { key: 'Enter' })
    await flushPromises()
    const download = [...document.querySelectorAll<HTMLElement>('[role="option"]')].at(-1)!
    download.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }))
    await flushPromises()
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'c', bubbles: true }))
    await flushPromises()
    expect(captures()[1]).toEqual({ capture_mode: 'standard', retain_image: false })
    const [[name, file]] = saveFile.mock.calls as unknown as [string, Blob][]
    expect(name).toMatch(/^capture-\d{4}-\d\d-\d\dT\d\d-\d\d-\d\dZ\.jpeg$/)
    expect(await file.text()).toBe('jpeg')
    expect(window.localStorage.getItem('microscope.capture-to')).toBe('download')
  })
})
