import { flushPromises, type VueWrapper } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { Router } from 'vue-router'

import type { LiveSequenceDetails } from '@/api/things'
import type { ThingDescription } from '@/api/wot/td'
import { useConnectionStore } from '@/connection/store'
import { clock } from '@/sequence/sequence'

import { fakeServer, json, type Received } from '../api/fakeServer'
import { mountApp } from '../mountApp'

/** A Thing's description, with properties of these schemas, and actions. */
function thing(name: string, properties: Record<string, object>, actions: string[] = []) {
  const href = (affordance: string) => `/api/v1/${name}/${affordance}`
  return {
    title: name,
    base: 'http://localhost/',
    properties: Object.fromEntries(
      Object.entries(properties).map(([p, schema]) => [
        p,
        { ...schema, forms: [{ href: href(p), op: ['readproperty', 'writeproperty'] }] },
      ]),
    ),
    actions: Object.fromEntries(
      actions.map((a) => [a, { forms: [{ href: href(a), op: ['invokeaction'] }] }]),
    ),
  } as ThingDescription
}

const invocation = (status: string, more: object = {}) => ({
  id: 'q',
  status,
  action: '/api/v1/sequence/run_sequence',
  href: 'http://localhost/api/v1/action_invocations/q',
  ...more,
})

const running = (more: Partial<LiveSequenceDetails> = {}): LiveSequenceDetails => ({
  name: 'lapse',
  points: 21,
  image_count: 3,
  skipped: 1,
  duration_seconds: 75.4,
  last_point_seconds: 600,
  latest_image: 'sequences/lapse/image_00004.jpeg',
  result: null,
  ...more,
})

let settings: Record<string, unknown>
let details: LiveSequenceDetails | null
let status: string
let output: string | undefined
let log: string[]
let ongoing: object[]
let received: Received[]
let wrapper: VueWrapper | undefined
let router: Router

beforeEach(() => {
  vi.useFakeTimers()
  settings = { duration: 5400, dt: 30, autofocus_enabled: false, autofocus_dz: 2000 }
  ;[details, status, output, log, ongoing] = [null, '', undefined, [], []]
  const server = fakeServer({
    'GET /api/v1/thing_descriptions/': {
      system: thing('system', { hostname: {} }),
      camera: thing('camera', {}),
      sequence: thing(
        'sequence',
        {
          duration: { type: 'number', minimum: 0, default: 600 },
          dt: { type: 'number', minimum: 0.5, default: 30 },
          autofocus_enabled: { type: 'boolean', default: false },
          autofocus_dz: { type: 'integer', minimum: 100, default: 2000 },
          latest_sequence_live_details: {},
        },
        ['run_sequence', 'download_zip'],
      ),
    },
    'GET /api/v1/system/hostname': 'lab-pc',
    ...Object.fromEntries(
      Object.keys(settings).flatMap((name) => [
        [`GET /api/v1/sequence/${name}`, () => json(settings[name])],
        [`PUT /api/v1/sequence/${name}`, () => json(null)],
      ]),
    ),
    'GET /api/v1/sequence/latest_sequence_live_details': () => json(details),
    'GET /api/v1/sequence/run_sequence': () => json(ongoing),
    'POST /api/v1/sequence/run_sequence': () => {
      status = 'running'
      return json(invocation('running'), 201)
    },
    'GET /api/v1/action_invocations/q': () =>
      json(
        invocation(status, {
          output,
          log: log.map((message) => ({ message })),
          error: { detail: 'a sequence is already running' },
        }),
      ),
    'DELETE /api/v1/action_invocations/q': () => json(null),
  })
  received = server.received
  vi.stubGlobal('fetch', server.fetch)
})

afterEach(() => {
  wrapper?.unmount()
  wrapper = undefined
  document.body.innerHTML = ''
  vi.unstubAllGlobals()
  vi.useRealTimers()
})

async function open() {
  ;({ wrapper, router } = await mountApp('/sequence'))
  await useConnectionStore().connect({ kind: 'local' })
  await vi.advanceTimersByTimeAsync(10)
}

const pane = () => wrapper!.get('.control-pane')
const button = (label: string) => wrapper!.findAll('button').find((b) => b.text() === label)
/** The pane's details, by their labels. */
function info() {
  const [labels, values] = ['dt', 'dd'].map((tag) =>
    wrapper!.findAll(`.sequence-page__info ${tag}`).map((element) => element.text()),
  )
  return Object.fromEntries(labels!.map((label, i) => [label, values![i]]))
}
const sent = (method: string, path: string) =>
  received.filter((r) => r.method === method && r.url.pathname.endsWith(path))

describe('the Sequence tab', () => {
  it('sets the duration as hours, minutes and seconds, the interval and the autofocus', async () => {
    await open()
    expect(pane().text()).toContain('Interval (s)')
    expect(pane().text()).toContain('Autofocus before each capture')
    expect(pane().text()).toContain('Autofocus range (steps)')
    const [hours, minutes, seconds, interval] = pane().findAll('input')
    expect([hours, minutes, seconds, interval].map((i) => i!.element.value)).toEqual([
      '1',
      '30',
      '0',
      '30',
    ])
    await minutes!.setValue('45')
    await minutes!.trigger('keydown', { key: 'Enter' })
    await flushPromises()
    expect(sent('PUT', '/duration').map((r) => JSON.parse(r.body!))).toEqual([6300])
  })

  it('starts a sequence, follows its progress and log, and cancels it', async () => {
    await open()
    await pane().findAll('input').at(-1)!.setValue('lapse')
    await button('Start Sequence')!.trigger('click')
    await flushPromises()
    expect(JSON.parse(sent('POST', '/run_sequence')[0]!.body!)).toEqual({ name: 'lapse' })
    // Until it has begun, the last sequence's details aren't shown as its own.
    details = running({ name: 'old', result: 'success' })
    await vi.advanceTimersByTimeAsync(1000)
    expect(pane().text()).not.toContain('old')

    details = running()
    log = ['Skipped 1 time point: the steps took longer than the interval']
    await vi.advanceTimersByTimeAsync(1000)
    expect(info()).toEqual({
      Sequence: 'lapse',
      Images: '3 of 20',
      'Time elapsed': '0:01:15',
      'Time remaining': '0:08:45',
      Skipped: '1 time point',
    })
    expect(wrapper!.find('.sequence-page__log').text()).toContain('Skipped 1 time point:')
    // The main view shows its latest image, and the pane a mini stream.
    const latest = wrapper!.get('.sequence-page__latest').attributes('src')
    expect(latest).toContain('/api/v1/data/sequences/lapse/image_00004.jpeg')
    expect(pane().find('.mini-stream').exists()).toBe(true)

    await button('Cancel')!.trigger('click')
    await flushPromises()
    expect(sent('DELETE', '/action_invocations/q')).toHaveLength(1)
    expect(button('Cancelling…')!.attributes('disabled')).toBeDefined()
    status = 'cancelled'
    details = running({ result: 'cancelled' })
    await vi.advanceTimersByTimeAsync(2000)
    expect(pane().text()).toContain('The sequence was cancelled.')
    expect(Object.keys(info())).not.toContain('Time remaining')
    expect(button('Download ZIP')).toBeDefined()
    await pane().get('a.sequence-page__link').trigger('click')
    await flushPromises()
    expect(router.currentRoute.value.name).toBe('gallery')
    expect(router.currentRoute.value.query).toEqual({ view: 'sequences/lapse' })
  })

  it('shows a sequence too short to be seen running once it ends, then closes', async () => {
    details = running({ name: 'old', result: 'success' })
    await open()
    await button('Start Sequence')!.trigger('click')
    ;[status, output] = ['completed', 'quick']
    await vi.advanceTimersByTimeAsync(1000)
    expect(pane().text()).not.toContain('old')
    details = running({ name: 'quick', points: 1, image_count: 1, skipped: 0, result: 'success' })
    await vi.advanceTimersByTimeAsync(1000)
    expect(pane().text()).toContain('The sequence has finished.')
    expect(info()).toMatchObject({ Sequence: 'quick', Images: '1 of 1' })
    await button('Close')!.trigger('click')
    expect(button('Start Sequence')).toBeDefined()
  })

  it('picks up a sequence started elsewhere', async () => {
    details = running()
    status = 'running'
    ongoing = [invocation('running')]
    await open()
    await vi.advanceTimersByTimeAsync(1000)
    expect(button('Start Sequence')).toBeUndefined()
    expect(info()).toMatchObject({ Sequence: 'lapse', Images: '3 of 20' })
    expect(button('Cancel')).toBeDefined()
  })

  it('says why a sequence didn’t start, and stays ready to start one', async () => {
    await open()
    await button('Start Sequence')!.trigger('click')
    status = 'error'
    await vi.advanceTimersByTimeAsync(1000)
    expect(document.body.textContent).toContain('The sequence didn’t start.')
    expect(button('Start Sequence')).toBeDefined()
  })

  it('shows times as hours, minutes and seconds', () => {
    expect([clock(3723), clock(59.6), clock(-2)]).toEqual(['1:02:03', '0:01:00', '0:00:00'])
  })
})
