import { type DOMWrapper, flushPromises, type VueWrapper } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { GalleryEntry } from '@/api/things'
import type { ThingDescription } from '@/api/wot/td'
import { useConnectionStore } from '@/connection/store'
import { hostKey } from '@/host'
import { createBrowserHost } from '@/host/browser'

import { fakeServer, json, type Received } from '../api/fakeServer'
import { mountApp } from '../mountApp'

/** A Thing's description, with readable properties and actions. */
function thing(name: string, properties: string[], actions: string[] = []): ThingDescription {
  const href = (affordance: string) => `/api/v1/${name}/${affordance}`
  return {
    title: name,
    base: 'http://localhost/',
    properties: Object.fromEntries(
      properties.map((p) => [p, { forms: [{ href: href(p), op: ['readproperty'] }] }]),
    ),
    actions: Object.fromEntries(
      actions.map((a) => [a, { forms: [{ href: href(a), op: ['invokeaction'] }] }]),
    ),
  }
}

/** Capture `n`, taken `n` seconds before New Year 2026. */
function capture(n: number, extra: Partial<GalleryEntry> = {}): GalleryEntry {
  const name = `capture-${n}`
  const path = `captures/${name}.jpeg`
  return {
    name,
    path,
    card_type: 'Capture',
    created: new Date(Date.UTC(2026, 0, 1) - n * 1000).toISOString(),
    thumbnail: `captures/thumbnails/${name}.jpeg`,
    viewer: { type: 'image', path },
    info: [{ label: 'Size', value: '64 × 48 px' }],
    download_buttons: [{ label: 'Image', source: 'data', path, filename: `${name}.jpeg` }],
    card_actions: [],
    card_messages: [],
    delete_endpoint: `gallery/captures/${name}.jpeg`,
    ...extra,
  }
}

const done = (id: string) =>
  json({ id, status: 'completed', href: `http://localhost/api/v1/action_invocations/${id}` }, 201)

let list: GalleryEntry[]
let received: Received[]
let wrapper: VueWrapper | undefined
const saveFile = vi.fn(async () => {})

beforeEach(() => {
  vi.useFakeTimers()
  list = Array.from({ length: 20 }, (_, i) => capture(i + 1))
  const server = fakeServer({
    'GET /api/v1/thing_descriptions/': {
      system: thing('system', ['hostname']),
      camera: thing('camera', []),
      gallery: thing(
        'gallery',
        ['list_data', 'card_types', 'bulk_actions'],
        ['delete_all_data', 'tidy'],
      ),
    },
    'GET /api/v1/system/hostname': 'lab-pc',
    'GET /api/v1/gallery/list_data': () => json(list),
    'GET /api/v1/gallery/card_types': ['Capture', 'Scan'],
    'GET /api/v1/gallery/bulk_actions': [{ thing: 'gallery', action: 'tidy', label: 'Tidy up' }],
    'POST /api/v1/gallery/tidy': () => done('t'),
    'POST /api/v1/gallery/delete_all_data': () => done('d'),
    'DELETE /api/v1/gallery/captures/capture-1.jpeg': () => new Response(null, { status: 204 }),
    'GET /api/v1/data/captures/capture-2.jpeg': () => new Response('image'),
    'GET /api/v1/data/captures/a%20b%23.jpeg': () => new Response('image'),
    'GET /api/v1/gallery/captures/capture-1.jpeg/zip': () => new Response('zip'),
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
  const host = { ...createBrowserHost(), saveFile }
  ;({ wrapper } = await mountApp('/gallery', { [hostKey]: host }))
  await useConnectionStore().connect({ kind: 'local' })
  await vi.advanceTimersByTimeAsync(0)
  await flushPromises()
}

const titles = () => wrapper!.findAll('.gallery-card__title').map((t) => t.text())
const showing = () => wrapper!.get('.gallery-page__showing').text()
const button = (label: string, within?: DOMWrapper<Element>) =>
  (within ?? wrapper!.get('.gallery-page'))
    .findAll('button')
    .find((b) => b.text() === label || b.attributes('aria-label') === label)
const all = (selector: string) => [...document.querySelectorAll<HTMLElement>(selector)]
const key = async (target: Element, key: string) => {
  target.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true }))
  await flushPromises()
}
const reads = () => received.filter((r) => r.url.pathname.endsWith('/list_data')).length
/** Answers the open question with its button `label`: the question's text. */
async function answer(label: string) {
  const question = document.querySelector<HTMLElement>('[role="alertdialog"]')!
  const text = question.textContent
  ;[...question.querySelectorAll('button')].find((b) => b.textContent!.trim() === label)!.click()
  await flushPromises()
  return text
}

describe('the Gallery tab', () => {
  it('shows the newest 18 cards, the rest on the next page, or 25 to a page', async () => {
    list.push(capture(21, { card_type: 'Scan', name: 'scan-21' }))
    await open()
    expect(showing()).toBe('Showing 1–18 of 21')
    expect(titles()).toEqual(list.slice(0, 18).map((e) => e.name))
    await wrapper!.get('nav[aria-label="Pages"] [aria-label="Next page"]').trigger('click')
    await flushPromises()
    expect(showing()).toBe('Showing 19–21 of 21')
    expect(titles()).toEqual(['capture-19', 'capture-20', 'scan-21'])

    await button('Show the sidebar')!.trigger('click')
    const [, perPage] = wrapper!.findAll('.gallery-page__sidebar [role="combobox"]')
    await perPage!.trigger('keydown', { key: 'Enter' })
    await flushPromises()
    await key(all('[role="option"]').at(-1)!, 'Enter')
    expect(showing()).toBe('Showing 1–21 of 21')
    expect(wrapper!.find('nav[aria-label="Pages"]').exists()).toBe(false)
  })

  it('shows a card’s thumbnail, info, messages and downloads, or thumbnails only', async () => {
    const warning = { level: 'warning' as const, message: 'Its metadata is missing.' }
    const zip = {
      label: 'Image and metadata',
      source: 'endpoint' as const,
      thing: 'gallery',
      path: 'captures/capture-1.jpeg/zip',
      filename: 'capture-1.zip',
    }
    list = [
      capture(1, {
        card_messages: [warning],
        download_buttons: [...capture(1).download_buttons, zip],
      }),
      capture(2, {
        thumbnail: null,
        download_buttons: [
          { label: 'Image', source: 'data', path: 'captures/a b#.jpeg', filename: 'a b#.jpeg' },
        ],
      }),
    ]
    await open()
    const [first, second] = wrapper!.findAll('.gallery-card')
    const image = first!.get('.gallery-card__thumbnail img')
    expect(new URL(image.attributes('src')!).pathname).toBe(
      '/api/v1/data/captures/thumbnails/capture-1.jpeg',
    )
    expect(second!.find('.gallery-card__thumbnail img').exists()).toBe(false)
    await image.trigger('error')
    expect(first!.find('.gallery-card__thumbnail img').exists()).toBe(false)
    const line = first!.get('.gallery-card__info div')
    expect([line.get('dt').text(), line.get('dd').text()]).toEqual(['Size', '64 × 48 px'])
    expect(first!.get('.gallery-card__message--warning').text()).toBe('Its metadata is missing.')
    expect(first!.text()).toContain(new Date(list[0]!.created).toLocaleString())

    // Several downloads are a menu; one is a button.
    const menu = button('Download', first)!
    menu.element.focus()
    await key(menu.element, 'Enter')
    await key(all('[role="menuitem"]')[1]!, 'Enter')
    await button('Image', second)!.trigger('click')
    await flushPromises()
    const saved = saveFile.mock.calls as unknown as [string, Blob][]
    expect(saved.map(([name]) => name)).toEqual(['capture-1.zip', 'a b#.jpeg'])

    await button('Show thumbnails only')!.trigger('click')
    expect(button('View', first)).toBeUndefined()
    expect(first!.find('.gallery-card__info').exists()).toBe(false)
    expect(titles()).toEqual(['capture-1', 'capture-2'])
  })

  it('deletes a card after asking, then reads the gallery again', async () => {
    await open()
    const before = reads()
    await button('Delete', wrapper!.findAll('.gallery-card')[0])!.trigger('click')
    await flushPromises()
    expect(await answer('Delete')).toContain('Delete capture-1 from the microscope?')
    const deleted = received.filter((r) => r.method === 'DELETE').map((r) => r.url.pathname)
    expect(deleted).toEqual(['/api/v1/gallery/captures/capture-1.jpeg'])
    expect(reads()).toBe(before + 1)
  })

  it('filters by type, runs bulk actions, and deletes all of the types shown', async () => {
    list.push(capture(21, { card_type: 'Scan', name: 'scan-21' }))
    await open()
    await button('Show the sidebar')!.trigger('click')
    const [types] = wrapper!.findAll('.gallery-page__sidebar [role="combobox"]')
    expect(types!.text()).toBe('Capture, Scan')
    await types!.trigger('keydown', { key: 'Enter' })
    await flushPromises()
    await key(all('[role="option"]')[0]!, 'Enter')
    await key(all('[role="option"]')[0]!, 'Escape')
    expect(titles()).toEqual(['scan-21'])
    expect(showing()).toBe('Showing 1–1 of 1')

    await button('Tidy up')!.trigger('click')
    await flushPromises()
    expect(received.some((r) => r.url.pathname.endsWith('/tidy'))).toBe(true)

    await button('Delete all')!.trigger('click')
    await flushPromises()
    const question = document.querySelector('[role="alertdialog"] ul')!
    expect([...question.querySelectorAll('li')].map((li) => li.textContent)).toEqual(['Scan'])
    await answer('Delete all')
    const posted = received.filter((r) => r.url.pathname.endsWith('/delete_all_data'))
    expect(posted.map((r) => JSON.parse(r.body!))).toEqual([{ card_types: ['Scan'] }])
  })

  it('reads the gallery again on Refresh, and shows an image with View', async () => {
    await open()
    expect(reads()).toBe(1)
    await button('Refresh')!.trigger('click')
    await flushPromises()
    expect(reads()).toBe(2)
    await button('View', wrapper!.findAll('.gallery-card')[1])!.trigger('click')
    await flushPromises()
    const viewer = document.querySelector('[role="dialog"]')!
    expect(viewer.textContent).toContain('capture-2')
    expect(new URL(viewer.querySelector('img')!.src).pathname).toBe(
      '/api/v1/data/captures/capture-2.jpeg',
    )
  })

  it('says when nothing has been saved', async () => {
    list = []
    await open()
    expect(wrapper!.text()).toContain('Nothing has been saved yet.')
    expect(showing()).toBe('')
  })
})
