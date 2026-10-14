import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { WotClient } from '@/api/wot/client'
import type { DataSchema, ThingDescription } from '@/api/wot/td'
import { useConnectionStore } from '@/connection/store'
import { hostKey } from '@/host'
import { joinDuration, splitDuration } from '@/server-ui/duration'
import ServerInterface from '@/server-ui/ServerInterface.vue'
import ServerPanel from '@/server-ui/ServerPanel.vue'
import type { UiElement } from '@/server-ui/types'
import { confirmKey, toastKey } from '@/ui/overlays'

import panel from '../../fixtures/server-ui/panel.json'
import { fakeServer, json, type Received } from '../api/fakeServer'

/** The camera's properties in the fixture, their schemas and values. */
const schemas: Record<string, DataSchema> = {
  exposure_time: { type: 'integer', minimum: 100, maximum: 1_000_000, default: 10_000 },
  repeating: { type: 'boolean', default: false },
  objective: { type: 'integer', enum: [4, 10, 20, 40, 60, 100], default: 40 },
  settling_time: { type: 'number', minimum: 0, maximum: 60, default: 0.2 },
  colour: { type: 'string', default: '#965aaa' },
  hostname: { type: 'string', readOnly: true },
}
let values: Record<string, unknown>
let received: Received[]
/** The status of the next PUT. */
let putStatus: number
/** Sends the observed exposure time a new value. */
let sendExposure: (value: number) => void
/** The statuses of the settle and capture invocations, which the tests move on. */
let settled: string
let captured: string
let capturedOutput: unknown
/** The answer to the next confirmation. */
let answer: boolean
/** How often the camera's panel has been fetched, and whether it's there. */
let panelFetches: number
let panelServed: boolean

const camera: ThingDescription = {
  title: 'SimulatedCamera',
  base: 'http://microscope.test/',
  properties: Object.fromEntries(
    Object.entries(schemas).map(([name, schema]) => [
      name,
      {
        ...schema,
        forms: [{ href: `/api/v1/camera/${name}`, op: ['readproperty', 'writeproperty'] }],
        observable: name === 'exposure_time',
      },
    ]),
  ),
  forms: [{ href: '/api/v1/camera/properties', op: ['observeallproperties'], subprotocol: 'sse' }],
  actions: Object.fromEntries(
    ['settle', 'capture_to_memory', 'grab_jpeg'].map((name) => [
      name,
      { forms: [{ href: `/api/v1/camera/${name}`, op: ['invokeaction'] }] },
    ]),
  ),
}

/** An invocation's record. */
const record = (id: string, status: string, more: object = {}) => ({
  id,
  status,
  href: `http://microscope.test/api/v1/action_invocations/${id}`,
  ...more,
})

/** The routes of the camera's actions, its file, and the blob an action returns. */
const actionRoutes = {
  'POST /api/v1/camera/settle': () => json(record('s', 'running'), 201),
  'GET /api/v1/action_invocations/s': () =>
    json(record('s', settled, { log: [{ message: 'Waiting for frames' }] })),
  'DELETE /api/v1/action_invocations/s': () => {
    settled = 'cancelled'
    return json(null)
  },
  'POST /api/v1/camera/capture_to_memory': () => json(record('c', 'running'), 201),
  'GET /api/v1/action_invocations/c': () =>
    json(record('c', captured, { output: capturedOutput, error: { detail: 'No camera.' } })),
  'POST /api/v1/camera/grab_jpeg': () =>
    json(record('g', 'completed', { output: { href: '/api/v1/blob/b1' } }), 201),
  'GET /api/v1/blob/b1': () => new Response('jpeg'),
  'GET /api/v1/camera/settings.json': () => new Response('{"a":1}'),
  'GET /api/v1/camera/manual_camera_settings': () => {
    panelFetches += 1
    if (!panelServed) return json({ detail: 'Not Found' }, 404)
    const settle = { kind: 'action_button', thing: 'camera', action: 'settle', label: 'Settle' }
    return json([
      { kind: 'header', text: [{ text: 'Camera' }] },
      { ...settle, refresh: true },
    ])
  },
}

/** The camera's stream of property changes, which sends the exposure time on demand. */
function propertyChanges(request: Received) {
  const body = new ReadableStream<Uint8Array>({
    start(controller) {
      const event = (value: number) => `event: exposure_time\ndata: ${value}\n\n`
      sendExposure = (value) => controller.enqueue(new TextEncoder().encode(event(value)))
      request.signal?.addEventListener('abort', () => controller.error(request.signal!.reason))
    },
  })
  return new Response(body, { headers: { 'Content-Type': 'text/event-stream' } })
}

const toaster = { success: vi.fn(), info: vi.fn(), error: vi.fn() }
const host = { openLink: vi.fn(), saveFile: vi.fn(async () => {}) }
const confirm = vi.fn(async () => answer)
let wrapper: VueWrapper | undefined

beforeEach(() => {
  vi.useFakeTimers()
  setActivePinia(createPinia())
  values = {
    exposure_time: 10_000,
    repeating: true,
    objective: 40,
    settling_time: 3725.5,
    colour: '#965aaa',
    hostname: 'lab-pc',
  }
  putStatus = 201
  ;[settled, captured, capturedOutput, answer] = ['running', 'running', 1, true]
  ;[panelFetches, panelServed] = [0, true]
  const routes = Object.fromEntries(
    Object.keys(schemas).flatMap((name) => [
      [`GET /api/v1/camera/${name}`, () => json(values[name])],
      [
        `PUT /api/v1/camera/${name}`,
        (request: Received) => {
          if (putStatus < 300) values[name] = JSON.parse(request.body ?? 'null')
          return json(putStatus < 300 ? null : { detail: 'Not allowed.' }, putStatus)
        },
      ],
    ]),
  )
  const server = fakeServer({
    ...routes,
    ...actionRoutes,
    'GET /api/v1/camera/properties': propertyChanges,
  })
  received = server.received
  const connection = useConnectionStore()
  connection.client = new WotClient({
    baseUrl: 'http://microscope.test/api/v1/',
    fetch: server.fetch,
  })
  connection.descriptions = { camera }
  connection.state = 'connected'
})

afterEach(() => {
  wrapper?.unmount()
  wrapper = undefined
  vi.clearAllMocks()
  vi.useRealTimers()
})

async function show(elements: UiElement[] = panel as UiElement[]) {
  wrapper = mount(ServerInterface, {
    props: { elements },
    global: { provide: { [toastKey]: toaster, [hostKey]: host, [confirmKey]: confirm } },
    attachTo: document.body,
  })
  await flushPromises()
  return wrapper
}

const control = (label: string) =>
  wrapper!.findAll('.property-control').find((c) => c.text().includes(label))!
const puts = () =>
  received
    .filter((r) => r.method === 'PUT')
    .map((r) => [r.url.pathname.split('/').pop(), JSON.parse(r.body ?? 'null')])

describe('ServerInterface', () => {
  it('draws the fixture’s text, sections and containers', async () => {
    await show()
    expect(wrapper!.find('h3').text()).toBe('Simulation')
    expect(wrapper!.find('em').text()).toBe('simulated')
    expect(wrapper!.findAll('li strong').map((b) => b.text())).toEqual(['Defaults'])
    await wrapper!.get('a').trigger('click')
    expect(host.openLink).toHaveBeenCalledWith('https://example.org/guide')
    expect(wrapper!.find('.server-interface--row').text()).toContain('Repeating sample')

    // The accordion opens to show its children.
    expect(wrapper!.text()).not.toContain('Colours')
    await wrapper!
      .findAll('button')
      .find((button) => button.text() === 'More')!
      .trigger('click')
    await flushPromises()
    expect(control('Colours').find('input').element.value).toBe('#965aaa')
    expect(control('Missing').text()).toContain('camera has no property “no_such_property”')
  })

  it('writes committed values, and reads back when asked', async () => {
    await show()
    const exposure = control('Exposure').get('input')
    await exposure.setValue('20000')
    await exposure.trigger('keydown', { key: 'Enter' })
    await control('Repeating').get('input').setValue(false)
    await flushPromises()
    expect(puts()).toEqual([
      ['exposure_time', 20_000],
      ['repeating', false],
    ])
    const reads = () => received.filter((r) => r.url.pathname.endsWith('/exposure_time')).length
    const before = reads()
    await vi.advanceTimersByTimeAsync(500)
    expect(reads()).toBe(before + 1)
  })

  it('follows observable properties, sharing one stream', async () => {
    await show([panel[3] as UiElement, panel[3] as UiElement])
    sendExposure(30_000)
    await vi.advanceTimersByTimeAsync(0)
    const fields = wrapper!.findAll('input').map((input) => input.element.value)
    expect(fields).toEqual(['30000', '30000'])
    expect(control('Exposure').classes()).toContain('property-control--changed')
    const streams = () => received.filter((r) => r.url.pathname.endsWith('/properties'))
    expect(streams()).toHaveLength(1)
    wrapper!.unmount()
    wrapper = undefined
    expect(streams()[0]!.signal?.aborted).toBe(true)
  })

  it('offers the panel’s options, and the schema’s enum', async () => {
    await show([
      panel[4]!.children![1] as UiElement,
      { kind: 'property_control', thing: 'camera', property: 'objective', label: 'Any' },
    ])
    const lists = wrapper!.findAll('[role="combobox"]')
    expect(lists.map((list) => list.text())).toEqual(['40×', '40'])
    await lists[0]!.trigger('keydown', { key: 'Enter' })
    await flushPromises()
    const options = [...document.querySelectorAll<HTMLElement>('[role="option"]')]
    expect(options.map((option) => option.textContent?.trim())).toEqual(['10×', '40×'])
    options[0]!.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }))
    await flushPromises()
    expect(puts()).toEqual([['objective', 10]])
  })

  it('writes text once it is committed, once', async () => {
    await show([panel[5]!.children![1] as UiElement])
    const colour = wrapper!.get('input')
    colour.element.value = '#000000' // typed: no change event until it's committed
    await colour.trigger('input')
    expect(puts()).toEqual([])
    await colour.trigger('keydown', { key: 'Enter' })
    await colour.trigger('change')
    await flushPromises()
    expect(puts()).toEqual([['colour', '#000000']])
  })

  it('enters a duration as hours, minutes and seconds', async () => {
    await show([panel[5]!.children![0] as UiElement])
    const parts = wrapper!.findAll('input')
    expect(parts.map((part) => part.element.value)).toEqual(['1', '2', '5.5'])
    await parts[1]!.setValue('3')
    await parts[1]!.trigger('keydown', { key: 'Enter' })
    await flushPromises()
    expect(puts()).toEqual([['settling_time', 3785.5]])
    expect(splitDuration(0.1 + 0.2)).toEqual({ hours: 0, minutes: 0, seconds: 0.3 })
    expect(joinDuration({ hours: 2, minutes: 0, seconds: 1 })).toBe(7201)
  })

  it('highlights a value changed from its default', async () => {
    await show()
    expect(control('Repeating').classes()).toContain('property-control--changed')
    expect(control('Repeating').text()).toContain('Changed from its default.')
    expect(control('Exposure').classes()).not.toContain('property-control--changed')
  })

  it('undoes and reports a write the Thing refuses', async () => {
    putStatus = 422
    await show()
    await control('Repeating').get('input').setValue(false)
    await flushPromises()
    expect(control('Repeating').get('input').element.checked).toBe(true)
    expect(toaster.error).toHaveBeenCalledWith("Couldn't set Repeating sample.", expect.anything())
  })

  it('shows read-only values, missing properties and unknown elements', async () => {
    await show([
      { kind: 'property_control', thing: 'camera', property: 'hostname', label: 'Host' },
      { kind: 'property_control', thing: 'camera', property: 'gain', label: 'Gain' },
      { kind: 'text', text: [{ text: 'not a link', href: 'javascript:alert(1)' }] },
      { kind: 'chart' } as unknown as UiElement,
    ])
    expect(control('Host').find('output').text()).toBe('lab-pc')
    expect(control('Gain').text()).toContain('camera has no property “gain”')
    expect(wrapper!.find('a').exists()).toBe(false)
    expect(wrapper!.text()).toContain('needs a newer version of the app')
  })
})

describe('server-described buttons', () => {
  const [buttons, downloads] = [panel[6], panel[7]].map(
    (row) => (row as { children: UiElement[] }).children,
  ) as [UiElement[], UiElement[]]
  const button = (label: string) => wrapper!.findAll('button').find((b) => b.text() === label)
  const posted = (action: string) =>
    received
      .filter((r) => r.method === 'POST' && r.url.pathname.endsWith(`/${action}`))
      .map((r) => JSON.parse(r.body ?? 'null'))

  it('runs an action with its input, and says what it returned', async () => {
    await show([buttons[1]!])
    await button('Capture')!.trigger('click')
    await flushPromises()
    expect(posted('capture_to_memory')).toEqual([{ buffer_max: 2 }])
    expect(button('Cancel')).toBeUndefined() // not cancellable
    captured = 'completed'
    await vi.advanceTimersByTimeAsync(1000)
    expect(toaster.success).toHaveBeenCalledWith('1')
  })

  it("says an output's message, if it has one", async () => {
    capturedOutput = { sample: true, message: '62.5% sample' }
    await show([buttons[1]!])
    await button('Capture')!.trigger('click')
    captured = 'completed'
    await vi.advanceTimersByTimeAsync(1000)
    expect(toaster.success).toHaveBeenCalledWith('62.5% sample')
  })

  it('asks first, then shows its log and stream in a dialog, and cancels', async () => {
    answer = false
    const shown = await show([buttons[0]!])
    await button('Settle')!.trigger('click')
    await flushPromises()
    expect(confirm).toHaveBeenCalledWith(
      expect.objectContaining({ message: 'Wait for the camera to settle?' }),
    )
    expect(posted('settle')).toEqual([])

    answer = true
    await button('Settle')!.trigger('click')
    await vi.advanceTimersByTimeAsync(200)
    const dialog = document.querySelector('[role="dialog"]')!
    expect(dialog.textContent).toContain('Waiting for frames')
    expect(dialog.querySelector('[aria-label="Live image"]')).not.toBeNull()
    const cancel = [...dialog.querySelectorAll('button')].find(
      (b) => b.textContent?.trim() === 'Cancel',
    )
    cancel!.click()
    await vi.advanceTimersByTimeAsync(1000)
    expect(received.some((r) => r.method === 'DELETE')).toBe(true)
    expect(document.querySelector('[role="dialog"]')).toBeNull()
    expect(toaster.error).not.toHaveBeenCalled()
    expect(shown.emitted('refresh')).toBeUndefined()
  })

  it('says it succeeded, and asks for the panel to be refreshed', async () => {
    const shown = await show([panel[6] as UiElement])
    await button('Settle')!.trigger('click')
    await flushPromises()
    settled = 'completed'
    await vi.advanceTimersByTimeAsync(1000)
    expect(toaster.success).toHaveBeenCalledWith('Settled.')
    expect(shown.emitted('refresh')).toHaveLength(1)
  })

  it('shows progress beside the button, and reports a failure', async () => {
    await show([{ ...buttons[1]!, cancellable: true } as UiElement])
    await button('Capture')!.trigger('click')
    await flushPromises()
    expect(button('Capture')!.attributes('disabled')).toBeDefined()
    expect(button('Cancel')).toBeDefined()
    captured = 'error'
    await vi.advanceTimersByTimeAsync(1000)
    expect(toaster.error).toHaveBeenCalledWith('Capture failed.', expect.anything())
    expect(button('Capture')!.attributes('disabled')).toBeUndefined()
  })

  it('downloads an action’s file, and an endpoint’s, and reports a failure', async () => {
    const gone = { ...downloads[1]!, broken: false, path: 'gone.json', label: 'Gone' }
    await show([downloads[0]!, { ...downloads[1]!, broken: false } as UiElement, gone as UiElement])
    await button('Gone')!.trigger('click')
    await flushPromises()
    expect(toaster.error).toHaveBeenCalledWith("Couldn't download camera.json.", expect.anything())
    await button('Download a frame')!.trigger('click')
    await flushPromises()
    expect(posted('grab_jpeg')).toEqual([{ stream_name: 'lores' }])
    await button('Download settings')!.trigger('click')
    await flushPromises()
    const saved = host.saveFile.mock.calls as unknown as [string, Blob][]
    const files = await Promise.all(saved.map(async ([name, blob]) => [name, await blob.text()]))
    expect(files).toEqual([
      ['frame.jpg', 'jpeg'],
      ['camera.json', '{"a":1}'],
    ])
  })

  it('says which buttons aren’t available', async () => {
    const zap = { kind: 'action_button', thing: 'camera', action: 'zap', label: 'Zap' }
    await show([...buttons, ...downloads, zap as UiElement])
    const notes = wrapper!.findAll('[class$="__broken"]').map((note) => note.text())
    expect(notes).toEqual([
      "Missing isn't available: camera has no action “no_such_action”.",
      "Download settings isn't available: camera has no file “settings.json”.",
      "Zap isn't available: camera has no action “zap”.",
    ])
  })
})

describe('ServerPanel', () => {
  const showPanel = async () => {
    wrapper = mount(ServerPanel, {
      props: { thing: 'camera', path: 'manual_camera_settings' },
      global: { provide: { [toastKey]: toaster, [hostKey]: host, [confirmKey]: confirm } },
      attachTo: document.body,
    })
    await flushPromises()
  }

  it('fetches a Thing’s panel, and fetches it again when an action asks', async () => {
    await showPanel()
    expect(wrapper!.find('h3').text()).toBe('Camera')
    expect(panelFetches).toBe(1)
    await wrapper!.get('button').trigger('click')
    await flushPromises()
    settled = 'completed'
    await vi.advanceTimersByTimeAsync(1000)
    expect(panelFetches).toBe(2)
  })

  it('says when the Thing has no such panel', async () => {
    panelServed = false
    await showPanel()
    expect(wrapper!.text()).toContain("This panel isn't available.")
  })
})
