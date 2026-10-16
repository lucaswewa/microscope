import { type DOMWrapper, flushPromises, type VueWrapper } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { Router } from 'vue-router'

import type { GalleryEntry } from '@/api/things'
import type { ThingDescription } from '@/api/wot/td'
import { useConnectionStore } from '@/connection/store'
import { hostKey } from '@/host'
import { createBrowserHost } from '@/host/browser'

import { fakeServer, json, type Received } from '../api/fakeServer'
import { mountApp } from '../mountApp'

// OpenSeadragon draws with WebGL, which the tests' DOM lacks: a fake records
// what it's asked. Its world holds a sequence's images (P52), each loaded
// when its test says so, and is placed on screen where `at` says (P53b).
const osd = vi.hoisted(() => {
  /** An image in the world: its URL, its opacity, and whether it's in. */
  type Item = { url: string; opacity: number; removed: boolean; load: () => void }
  type Rect = { x: number; y: number; width: number; height: number }
  const viewers: {
    options: { tileSources: unknown }
    viewport: Record<'zoomBy' | 'applyConstraints' | 'goHome', ReturnType<typeof vi.fn>>
    items: Item[]
    /** Where the world's images are, in the view's pixels. */
    at: Rect
    destroy: ReturnType<typeof vi.fn>
    fire: (event: string) => void
  }[] = []
  type Added = { tileSource: { url: string }; opacity: number; success: (e: object) => void }
  const create = vi.fn((options: (typeof viewers)[number]['options']) => {
    const handlers: Record<string, () => void> = {}
    const items: Item[] = []
    const tiled = (item: Item) => ({
      whenFullyLoaded: (callback: () => void) => (item.load = callback),
      setOpacity: (value: number) => (item.opacity = value),
      getOpacity: () => item.opacity,
      getBounds: () => item,
      item,
    })
    const inWorld = () => items.filter((item) => !item.removed)
    const viewer = {
      options,
      viewport: {
        zoomBy: vi.fn(),
        applyConstraints: vi.fn(),
        goHome: vi.fn(),
        viewportToViewerElementRectangle: () => ({ ...viewer.at }),
      },
      items,
      at: { x: 0, y: 0, width: 100, height: 100 },
      addTiledImage: ({ tileSource, opacity, success }: Added) => {
        const item: Item = { url: tileSource.url, opacity, removed: false, load: () => {} }
        items.push(item)
        success({ item: tiled(item) })
      },
      world: {
        removeItem: ({ item }: { item: Item }) => (item.removed = true),
        getItemCount: () => inWorld().length,
        getItemAt: (index: number) => tiled(inWorld()[index]!),
      },
      destroy: vi.fn(),
      addHandler: (event: string, handler: () => void) => (handlers[event] = handler),
      fire: (event: string) => handlers[event]?.(),
    }
    // One image or a DZI is opened at once.
    const [source] = options.tileSources as (string | { url: string })[]
    if (source) items.push({ url: String(source), opacity: 1, removed: false, load: () => {} })
    viewers.push(viewer)
    return viewer
  })
  return { create, viewers }
})
vi.mock('openseadragon', () => ({ default: osd.create }))

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
let fullScreen: (active: boolean) => void
const setFullScreen = vi.fn(async () => {})
let received: Received[]
let wrapper: VueWrapper | undefined
let router: Router
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
    'GET /api/v1/data/scans/s/stitched/tiles.json': {
      size: [300, 100],
      tiles: [
        { image: 'img_0_0_5.jpeg', number: 1, x: 0, y: 0, width: 200, height: 100 },
        { image: 'img_9_0_5.jpeg', number: 2, x: 100, y: 0, width: 200, height: 100 },
      ],
    },
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

async function open(path = '/gallery') {
  const fullscreen = {
    active: () => false,
    set: setFullScreen,
    watch: (onChange: (active: boolean) => void) => ((fullScreen = onChange), () => {}),
  }
  const host = { ...createBrowserHost(), saveFile, fullscreen }
  ;({ wrapper, router } = await mountApp(path, { [hostKey]: host }))
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

  it('reads the gallery again on Refresh', async () => {
    await open()
    expect(reads()).toBe(1)
    await button('Refresh')!.trigger('click')
    await flushPromises()
    expect(reads()).toBe(2)
  })

  it('says when nothing has been saved', async () => {
    list = []
    await open()
    expect(wrapper!.text()).toContain('Nothing has been saved yet.')
    expect(showing()).toBe('')
  })
})

describe('the viewer', () => {
  /** Opens card `index`'s viewer: the dialog, and its fake OpenSeadragon. */
  async function view(index: number) {
    await button('View', wrapper!.findAll('.gallery-card')[index])!.trigger('click')
    await flushPromises()
    const dialog = document.querySelector<HTMLElement>('[role="dialog"]')!
    return { dialog, viewer: osd.viewers.at(-1)! }
  }
  const press = (label: string) =>
    document.querySelector<HTMLElement>(`[role="dialog"] button[aria-label="${label}"]`)!.click()
  const path = (url: unknown) => new URL(url as string).pathname

  it('shows an image to zoom and pan, and closes', async () => {
    await open()
    const { dialog, viewer } = await view(1)
    expect(dialog.querySelector('h2')!.textContent).toBe('capture-2')
    const [source] = viewer.options.tileSources as { type: string; url: string }[]
    expect([source!.type, path(source!.url)]).toEqual([
      'image',
      '/api/v1/data/captures/capture-2.jpeg',
    ])
    const status = () => dialog.querySelector('.deep-zoom')!.getAttribute('data-status')
    expect(status()).toBe('loading')
    viewer.fire('open')
    await flushPromises()
    expect(status()).toBe('shown')
    press('Zoom in')
    press('Zoom out')
    press('Show it all')
    expect(viewer.viewport.zoomBy.mock.calls).toEqual([[1.5], [1 / 1.5]])
    expect(viewer.viewport.goHome).toHaveBeenCalledOnce()
    press('Close')
    await flushPromises()
    expect(viewer.destroy).toHaveBeenCalledOnce()
    expect(document.querySelector('[role="dialog"]')).toBeNull()
  })

  it('opens the item a link names, and forgets the link once closed', async () => {
    await open('/gallery?view=captures/capture-2.jpeg')
    const dialog = document.querySelector<HTMLElement>('[role="dialog"]')!
    expect(dialog.querySelector('h2')!.textContent).toBe('capture-2')
    press('Close')
    await flushPromises()
    expect(document.querySelector('[role="dialog"]')).toBeNull()
    expect(router.currentRoute.value.fullPath).toBe('/gallery')
  })

  it('shows a deep-zoom image, and a sequence with a slider and playback', async () => {
    const paths = ['a.jpeg', 'b.jpeg', 'c.jpeg']
    list = [
      capture(1, { viewer: { type: 'dzi', path: 'scans/s/stitched.dzi' } }),
      capture(2, { viewer: { type: 'directory', paths, start: 1 } }),
    ]
    await open()
    const first = await view(0)
    const dzi = first.viewer.options.tileSources as string[]
    expect(dzi.map(path)).toEqual(['/api/v1/data/scans/s/stitched.dzi'])
    first.viewer.fire('open-failed')
    await flushPromises()
    expect(first.dialog.textContent).toContain("It couldn't be shown.")
    expect(first.dialog.querySelector('[aria-label="Image"]')).toBeNull()
    press('Close')
    await flushPromises()

    const { dialog, viewer } = await view(1)
    const status = () => dialog.querySelector('.deep-zoom')!
    // The images, hidden, then each shown once it's loaded (P52).
    const shown = () => viewer.items.filter((i) => !i.removed).map((i) => [path(i.url), i.opacity])
    expect(viewer.options.tileSources).toEqual([])
    expect(shown()).toEqual([['/api/v1/data/b.jpeg', 0]])
    expect(status().getAttribute('data-status')).toBe('loading')
    viewer.items[0]!.load()
    await flushPromises()
    expect(status().getAttribute('data-status')).toBe('shown')
    // Shown, with the next loaded ahead, hidden.
    expect(shown()).toEqual([
      ['/api/v1/data/b.jpeg', 1],
      ['/api/v1/data/c.jpeg', 0],
    ])
    expect(dialog.textContent).toContain('2 of 3')

    // The next stays hidden until it's loaded, and the one before stays shown.
    await key(dialog.querySelector('[aria-label="Image"]')!, 'ArrowRight')
    expect(dialog.textContent).toContain('3 of 3')
    expect(shown()).toEqual([
      ['/api/v1/data/b.jpeg', 1],
      ['/api/v1/data/c.jpeg', 0],
    ])
    expect(status().getAttribute('data-page')).toBe('1')
    viewer.items[1]!.load()
    await flushPromises()
    expect(shown()).toEqual([['/api/v1/data/c.jpeg', 1]])
    expect(status().getAttribute('data-page')).toBe('2')
    // The zoom and pan stay as they were.
    expect(viewer.viewport.goHome).not.toHaveBeenCalled()

    // Playing starts from the first, as the last is shown, and stops at the end.
    press('Play')
    await flushPromises()
    expect(dialog.textContent).toContain('1 of 3')
    expect(dialog.querySelector('[aria-label="Pause"]')).not.toBeNull()
    for (const item of viewer.items.slice(2)) item.load()
    await flushPromises()
    await vi.advanceTimersByTimeAsync(500)
    expect(dialog.textContent).toContain('2 of 3')
    for (const item of viewer.items.slice(3)) item.load()
    await vi.advanceTimersByTimeAsync(1000)
    expect(dialog.textContent).toContain('3 of 3')
    expect(dialog.querySelector('[aria-label="Play"]')).not.toBeNull()
  })

  it('outlines a stitched scan’s images over it, following the view', async () => {
    const stitched = { type: 'dzi', path: 'scans/s/stitched/mosaic.dzi' } as const
    list = [
      capture(1, { viewer: { ...stitched, tiles: 'scans/s/stitched/tiles.json' } }),
      capture(2, { viewer: stitched }),
    ]
    await open()
    const { dialog, viewer } = await view(0)
    // Drawn at twice the size, 10 pixels in and 20 down (P53b).
    viewer.at = { x: 10, y: 20, width: 600, height: 200 }
    viewer.fire('open')
    await flushPromises()
    const outlines = () => [...dialog.querySelectorAll('.overlay-layers__stroke')]
    const left = () => outlines().map((rect) => Number(rect.getAttribute('x')))
    expect(left()).toEqual([10, 210])
    expect(dialog.querySelector('.overlay-layers title')!.textContent).toBe('1: img_0_0_5.jpeg')
    // Zoomed in twice as far, and panned: they follow.
    viewer.at = { x: -290, y: 20, width: 1200, height: 400 }
    viewer.fire('update-viewport')
    await flushPromises()
    expect(left()).toEqual([-290, 110])
    expect(Number(outlines()[0]!.getAttribute('width'))).toBe(800)

    expect(dialog.textContent).toContain('Layers')

    // A mosaic stitched before P53, without its layout, has no Layers menu.
    press('Close')
    await flushPromises()
    const older = await view(1)
    expect(older.dialog.textContent).not.toContain('Layers')
  })

  it('changes the brightness, contrast and saturation, and resets them', async () => {
    await open()
    const { dialog } = await view(0)
    const filter = () => dialog.querySelector<HTMLElement>('.deep-zoom__view')!.style.filter
    expect(filter()).toBe('brightness(100%) contrast(100%) saturate(100%)')
    await key(dialog.querySelector('[aria-label="Brightness"]')!, 'PageUp')
    await key(dialog.querySelector('[aria-label="Contrast"]')!, 'ArrowLeft')
    await key(dialog.querySelector('[aria-label="Saturation"]')!, 'Home')
    expect(filter()).toBe('brightness(110%) contrast(99%) saturate(0%)')
    const reset = [...dialog.querySelectorAll('button')].find(
      (b) => b.textContent!.trim() === 'Reset',
    )
    reset!.click()
    await flushPromises()
    expect(filter()).toBe('brightness(100%) contrast(100%) saturate(100%)')
  })

  it('fills the screen, and leaves it when the screen does', async () => {
    await open()
    const { dialog } = await view(0)
    press('Full screen')
    await flushPromises()
    expect(setFullScreen).toHaveBeenLastCalledWith(true)
    expect(dialog.classList).toContain('app-dialog--full')
    fullScreen(false)
    await flushPromises()
    expect(dialog.classList).not.toContain('app-dialog--full')
    press('Full screen')
    await flushPromises()
    press('Close')
    await flushPromises()
    expect(setFullScreen).toHaveBeenLastCalledWith(false)
  })
})
