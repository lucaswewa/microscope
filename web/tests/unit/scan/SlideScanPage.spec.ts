import { flushPromises, type VueWrapper } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { LiveScanDetails } from '@/api/things'
import type { ThingDescription } from '@/api/wot/td'
import { useConnectionStore } from '@/connection/store'

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
  id: 's',
  status,
  action: '/api/v1/smart_scan/sample_scan',
  href: 'http://localhost/api/v1/action_invocations/s',
  ...more,
})

const scanning = (phase: LiveScanDetails['scan_phase'], more: Partial<LiveScanDetails> = {}) => ({
  name: 'slide-1',
  scan_phase: phase,
  image_count: 3,
  duration_seconds: 75.4,
  workflow: 'snake_workflow',
  settings: { overlap: 0.35, x_count: 3 },
  result: null,
  preview_stitch_time: null,
  ...more,
})

let workflow: string
let automatic: boolean
let details: LiveScanDetails | null
let status: string
let log: string[]
let ongoing: object[]
let received: Received[]
let wrapper: VueWrapper | undefined

beforeEach(() => {
  vi.useFakeTimers()
  ;[workflow, automatic, details, status, log, ongoing] = ['snake_workflow', true, null, '', [], []]
  const server = fakeServer({
    'GET /api/v1/thing_descriptions/': {
      system: thing('system', { hostname: {} }),
      camera: thing('camera', {}),
      smart_scan: thing(
        'smart_scan',
        {
          workflow_name: {},
          workflow_display_names: {},
          workflow_descriptions: {},
          latest_scan_live_details: {},
          stitch_automatically: { type: 'boolean', default: true },
        },
        ['sample_scan', 'download_zip'],
      ),
      snake_workflow: thing('snake_workflow', {}),
      raster_workflow: thing('raster_workflow', {}),
    },
    'GET /api/v1/system/hostname': 'lab-pc',
    'GET /api/v1/smart_scan/workflow_name': () => json(workflow),
    'PUT /api/v1/smart_scan/workflow_name': (r) => {
      workflow = JSON.parse(r.body!)
      return json(null)
    },
    'GET /api/v1/smart_scan/workflow_display_names': {
      raster_workflow: 'Raster scan',
      snake_workflow: 'Snake scan',
    },
    'GET /api/v1/smart_scan/workflow_descriptions': {
      raster_workflow: 'Row by row.',
      snake_workflow: 'Back and forth.',
    },
    'GET /api/v1/smart_scan/stitch_automatically': () => json(automatic),
    'GET /api/v1/smart_scan/latest_scan_live_details': () => json(details),
    'GET /api/v1/smart_scan/sample_scan': () => json(ongoing),
    'POST /api/v1/smart_scan/sample_scan': () => {
      status = 'running'
      return json(invocation('running'), 201)
    },
    'GET /api/v1/action_invocations/s': () =>
      json(
        invocation(status, {
          log: log.map((message) => ({ message })),
          error: { detail: 'Snake scan isn’t ready to scan' },
        }),
      ),
    'DELETE /api/v1/action_invocations/s': () => json(null),
    'GET /api/v1/snake_workflow/settings_ui': [{ kind: 'header', text: [{ text: 'Snake panel' }] }],
    'GET /api/v1/raster_workflow/settings_ui': [
      { kind: 'header', text: [{ text: 'Raster panel' }] },
    ],
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
  ;({ wrapper } = await mountApp('/slide-scan'))
  await useConnectionStore().connect({ kind: 'local' })
  await vi.advanceTimersByTimeAsync(10)
}

const pane = () => wrapper!.get('.control-pane')
const button = (label: string) => wrapper!.findAll('button').find((b) => b.text() === label)
const posted = () =>
  received.filter((r) => r.method === 'POST' && r.url.pathname.endsWith('/sample_scan'))

describe('the Slide Scan tab', () => {
  it('chooses a workflow, saying what it does, with its settings', async () => {
    automatic = false
    await open()
    expect(pane().get('[role="combobox"]').text()).toContain('Snake scan')
    expect(pane().text()).toContain('Back and forth.')
    expect(pane().text()).toContain('Snake panel')
    // Not stitching automatically is a change, shown on the closed section.
    expect(pane().text()).toContain('Stitching settings (changed)')

    await pane().get('[role="combobox"]').trigger('keydown', { key: 'Enter' })
    await flushPromises()
    const raster = [...document.querySelectorAll<HTMLElement>('[role="option"]')].find((o) =>
      o.textContent?.includes('Raster scan'),
    )!
    raster.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }))
    await flushPromises()
    expect(workflow).toBe('raster_workflow')
    expect(pane().text()).toContain('Row by row.')
    expect(pane().text()).toContain('Raster panel')
  })

  it('starts a scan, follows its phases and log, and cancels it', async () => {
    await open()
    await pane().get('input').setValue('slide-1')
    await button('Start Smart Scan')!.trigger('click')
    await flushPromises()
    expect(JSON.parse(posted()[0]!.body!)).toEqual({ scan_name: 'slide-1' })
    // Until it has begun, the last scan's details aren't shown as its own.
    details = scanning('complete', { name: 'old', result: 'success' })
    await vi.advanceTimersByTimeAsync(1000)
    expect(pane().text()).not.toContain('old')

    details = scanning('setup', { image_count: 0 })
    log = ['Scanning with snake_workflow from x 0, y 0, z 0']
    await vi.advanceTimersByTimeAsync(1000)
    expect(pane().text()).toContain('slide-1')
    expect(button('Cancel setup')).toBeDefined()
    expect(wrapper!.find('.slide-scan__log').text()).toContain('Scanning with snake_workflow')

    details = scanning('scanning', { preview_stitch_time: '2026-10-14T10:00:00.000Z' })
    await vi.advanceTimersByTimeAsync(1000)
    expect(pane().text()).toMatch(/Images captured\s*3/)
    expect(pane().text()).toMatch(/Duration\s*1:15/)
    const preview = wrapper!.get('.slide-scan__preview').attributes('src')
    expect(preview).toContain('/api/v1/smart_scan/latest_preview_stitch.jpg?drawn=2026-10-14')
    await button('Cancel scan')!.trigger('click')
    await flushPromises()
    expect(received.some((r) => r.method === 'DELETE')).toBe(true)
    expect(button('Cancelling…')!.attributes('disabled')).toBeDefined()

    status = 'cancelled'
    details = scanning('complete', { result: 'cancelled' })
    await vi.advanceTimersByTimeAsync(2000)
    expect(pane().text()).toContain('The scan was cancelled.')
    expect(button('Download ZIP')).toBeDefined()
    await button('Close')!.trigger('click')
    expect(button('Start Smart Scan')).toBeDefined()
  })

  it('shows the scan’s settings in a dialog', async () => {
    await open()
    await button('Start Smart Scan')!.trigger('click')
    details = scanning('scanning')
    await vi.advanceTimersByTimeAsync(1000)
    await wrapper!.get('[aria-label="The scan\'s settings"]').trigger('click')
    await flushPromises()
    const dialog = document.querySelector('[role="dialog"]')!
    expect(dialog.textContent).toContain('x_count')
    expect(dialog.textContent).toContain('0.35')
  })

  it('picks up a scan started elsewhere', async () => {
    details = scanning('scanning')
    status = 'running'
    log = ['Moving to x 0, y 0, z 0']
    ongoing = [invocation('running')]
    await open()
    await vi.advanceTimersByTimeAsync(1000)
    expect(button('Start Smart Scan')).toBeUndefined()
    expect(pane().text()).toContain('slide-1')
    expect(wrapper!.find('.slide-scan__log').text()).toContain('Moving to x 0')
    expect(button('Cancel scan')).toBeDefined()
  })

  it('says why a scan didn’t start, and stays ready to start one', async () => {
    await open()
    await button('Start Smart Scan')!.trigger('click')
    status = 'error'
    await vi.advanceTimersByTimeAsync(1000)
    expect(document.body.textContent).toContain('The scan didn’t start.')
    expect(button('Start Smart Scan')).toBeDefined()
  })
})
