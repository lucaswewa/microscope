import { flushPromises, type VueWrapper } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { ThingDescription } from '@/api/wot/td'
import { useConnectionStore } from '@/connection/store'
import { hostKey } from '@/host'
import { createBrowserHost } from '@/host/browser'
import type { ServerLogRecord } from '@/logging/log'

import { fakeServer, json, type Received } from '../api/fakeServer'
import { mountApp } from '../mountApp'
import { LOG, record } from './fixture'

const thing = (title: string, properties: string[] = []): ThingDescription => ({
  title,
  base: 'http://localhost/',
  properties: Object.fromEntries(
    properties.map((p) => [
      p,
      { forms: [{ href: `/api/v1/${title}/${p}`, op: ['readproperty'] }] },
    ]),
  ),
})

let log: ServerLogRecord[]
let logFile: Response
let received: Received[]
let wrapper: VueWrapper | undefined
const saveFile = vi.fn<(name: string, data: Blob) => Promise<void>>(async () => {})

beforeEach(() => {
  vi.useFakeTimers()
  log = LOG
  logFile = new Response('2026-10-14T00:00:01Z [INFO] serving')
  const server = fakeServer({
    'GET /api/v1/thing_descriptions/': {
      system: thing('system', ['hostname']),
      camera: thing('camera'),
    },
    'GET /api/v1/system/hostname': 'lab-pc',
    'GET /api/v1/log/': () => json(log),
    'GET /api/v1/logfile/': () => logFile,
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

async function open() {
  ;({ wrapper } = await mountApp('/logging', { [hostKey]: { ...createBrowserHost(), saveFile } }))
  await useConnectionStore().connect({ kind: 'local' })
  await vi.advanceTimersByTimeAsync(0)
  await flushPromises()
}

const groups = () => wrapper!.findAll('details.log-group')
const summaries = () =>
  groups().map((g) => [g.get('.log-group__title').text(), g.get('.log-level').text()])
const button = (label: string) => wrapper!.findAll('button').find((b) => b.text() === label)!
async function chooseLevel(name: string) {
  await wrapper!.get('.logging-page__level [role="combobox"]').trigger('keydown', { key: 'Enter' })
  await flushPromises()
  const option = [...document.querySelectorAll<HTMLElement>('[role="option"]')].find(
    (o) => o.textContent?.trim() === name,
  )!
  option.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }))
  await flushPromises()
}

describe('the Logging tab', () => {
  it('shows the newest groups first, at INFO and above, each with its most severe level', async () => {
    await open()
    expect(summaries()).toEqual([
      ['microscope_server::lifecycle', 'INFO'],
      ['Invocation c3', 'INFO'],
      ['sequence.run_sequence', 'ERROR'],
      ['autofocus.fast_autofocus', 'WARNING'],
      ['microscope_server::lifecycle', 'INFO'],
      ['microscope_server::cli', 'INFO'],
    ])
    expect(groups()[2]!.get('.log-group__meta').text()).toContain('3 records')
  })

  it('opens a group to show its records, and expands and collapses them all', async () => {
    await open()
    const sequence = groups()[2]!
    expect(sequence.find('.log-record').exists()).toBe(true) // rendered, but closed
    expect(sequence.attributes('open')).toBeUndefined()
    const details = sequence.element as HTMLDetailsElement
    details.open = true
    await sequence.trigger('toggle')
    const records = sequence.findAll('.log-record')
    expect(records.map((r) => r.get('.log-record__message').text())).toEqual([
      'running run_sequence wot::things',
      'Skipped 4 time points microscope_things::sequence',
      "the captures can't be saved wot::things",
    ])
    expect(records[2]!.get('.log-record__error').text()).toBe('ActionError\nthe disk is full')

    await button('Expand all').trigger('click')
    expect(groups().every((g) => g.attributes('open') !== undefined)).toBe(true)
    await button('Collapse all').trigger('click')
    expect(groups().every((g) => g.attributes('open') === undefined)).toBe(true)
  })

  it('hides less severe records at a level, and says when nothing is left', async () => {
    await open()
    await chooseLevel('WARNING')
    expect(summaries()).toEqual([
      ['sequence.run_sequence', 'ERROR'],
      ['autofocus.fast_autofocus', 'WARNING'],
    ])
    expect(groups()[0]!.get('.log-group__meta').text()).toContain('2 records')
    await chooseLevel('CRITICAL')
    expect(groups()).toHaveLength(0)
    expect(wrapper!.text()).toContain('Nothing at CRITICAL or above.')
    await chooseLevel('DEBUG')
    expect(groups()).toHaveLength(8)
  })

  it('shows 10 groups to a page', async () => {
    // 25 groups: records from two sources, in turn.
    log = Array.from({ length: 25 }, (_, i) => record(i + 1, 'INFO', `source::${i % 2}`, `m${i}`))
    await open()
    const first = () => groups()[0]!.get('.log-record__message').text()
    expect([groups().length, first()]).toEqual([10, 'm24'])
    await wrapper!.get('nav[aria-label="Pages"] [aria-label="Next page"]').trigger('click')
    await flushPromises()
    expect([groups().length, first()]).toEqual([10, 'm14'])
    // Another level starts again from the first page.
    await chooseLevel('DEBUG')
    expect(first()).toBe('m24')
  })

  it('says when the log is empty, and reads it again on Refresh', async () => {
    log = []
    await open()
    expect(wrapper!.text()).toContain('The log is empty.')
    log = LOG
    await button('Refresh').trigger('click')
    await flushPromises()
    expect(groups()).toHaveLength(6)
    expect(received.filter((r) => r.url.pathname === '/api/v1/log/')).toHaveLength(2)
  })

  it('downloads today’s log file, or says why it couldn’t', async () => {
    await open()
    await button('Download log file').trigger('click')
    await flushPromises()
    const [name, data] = saveFile.mock.calls[0]!
    expect(name).toMatch(/^microscope\.\d{4}-\d{2}-\d{2}\.log$/)
    expect(await data.text()).toContain('serving')

    logFile = json({ detail: 'No log file has been written.' }, 404)
    await button('Download log file').trigger('click')
    await flushPromises()
    expect(document.body.textContent).toContain("Couldn't download the log file.")
  })
})
