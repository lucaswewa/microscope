import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { WotClient } from '@/api/wot/client'
import type { DataSchema, ThingDescription } from '@/api/wot/td'
import { useConnectionStore } from '@/connection/store'
import { hostKey } from '@/host'
import { joinDuration, splitDuration } from '@/server-ui/duration'
import ServerInterface from '@/server-ui/ServerInterface.vue'
import type { UiElement } from '@/server-ui/types'
import { toastKey } from '@/ui/overlays'

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
const host = { openLink: vi.fn() }
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
  const server = fakeServer({ ...routes, 'GET /api/v1/camera/properties': propertyChanges })
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
    global: { provide: { [toastKey]: toaster, [hostKey]: host } },
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
