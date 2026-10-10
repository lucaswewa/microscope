import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { Component } from 'vue'

import { WotClient } from '@/api/wot/client'
import type { ThingDescription } from '@/api/wot/td'
import { useConnectionStore } from '@/connection/store'
import { flashLiveImage } from '@/live/flash'
import LiveImage from '@/live/LiveImage.vue'
import { STREAM_DISABLED_KEY } from '@/live/preferences'
import ViewPage from '@/views/ViewPage.vue'

import { fakeServer, jpeg, json, mjpegStream, type Received } from '../api/fakeServer'

const camera: ThingDescription = {
  title: 'SimulatedCamera',
  base: 'http://microscope.test/',
  links: [{ href: '/api/v1/camera/mjpeg_stream', type: 'multipart/x-mixed-replace' }],
}

/** Stands in for IntersectionObserver: tests say whether the image is on screen. */
let onScreen: (visible: boolean) => void = () => {}
class FakeObserver {
  constructor(private readonly callback: IntersectionObserverCallback) {
    onScreen = (visible) =>
      this.callback(
        [{ isIntersecting: visible } as IntersectionObserverEntry],
        this as unknown as IntersectionObserver,
      )
  }
  observe() {}
  disconnect() {}
}

let wrapper: VueWrapper | undefined
let requests: Received[]
let send: (n: number) => void
let failures: number

beforeEach(() => {
  vi.useFakeTimers()
  window.localStorage.clear()
  setActivePinia(createPinia())
  vi.stubGlobal('IntersectionObserver', FakeObserver)
  vi.stubGlobal(
    'createImageBitmap',
    vi.fn(async () => ({ width: 820, height: 616, close: vi.fn() })),
  )
  requests = []
  failures = 0
})

afterEach(() => {
  wrapper?.unmount()
  wrapper = undefined
  vi.unstubAllGlobals()
  vi.useRealTimers()
})

/** Connects the store to a camera, which fails `failures` times before streaming. */
function connect() {
  const server = fakeServer({
    'GET /api/v1/camera/mjpeg_stream': (request: Received) => {
      requests.push(request)
      if (requests.length <= failures) return json({ detail: 'The camera is starting.' }, 503)
      const stream = mjpegStream(request)
      send = (n) => stream.push(jpeg(n))
      return stream.response
    },
  })
  const connection = useConnectionStore()
  connection.client = new WotClient({
    baseUrl: 'http://microscope.test/api/v1/',
    fetch: server.fetch,
  })
  connection.descriptions = { camera }
  connection.state = 'connected'
}

async function show(component: Component = LiveImage) {
  wrapper = mount(component, { attachTo: document.body })
  await vi.advanceTimersByTimeAsync(0)
  await flushPromises()
  return wrapper
}

const status = () => wrapper!.find('.live-image').attributes('data-status')
const message = () => wrapper!.find('[role="status"]').text()

describe('LiveImage', () => {
  it('draws the stream’s frames, and says why while there are none', async () => {
    await show()
    expect(status()).toBe('offline')
    expect(message()).toContain('No connection')
    connect()
    await vi.advanceTimersByTimeAsync(0)
    expect(status()).toBe('connecting')
    expect(message()).toContain('Connecting')
    send(1)
    send(2)
    await vi.advanceTimersByTimeAsync(200)
    expect(status()).toBe('live')
    expect(wrapper!.find('[role="status"]').exists()).toBe(false)
    const canvas = wrapper!.find('canvas')
    expect(canvas.attributes('data-frames')).toBe('2')
    expect([canvas.element.width, canvas.element.height]).toEqual([820, 616])
  })

  it('shows an error while the stream fails, then the frames', async () => {
    failures = 1
    connect()
    await show()
    expect(status()).toBe('error')
    expect(message()).toContain('The camera is starting.')
    await vi.advanceTimersByTimeAsync(1000)
    send(1)
    await vi.advanceTimersByTimeAsync(0)
    expect(status()).toBe('live')
  })

  it('pauses its stream while off screen or while the page is hidden', async () => {
    connect()
    await show()
    expect(requests).toHaveLength(1)
    onScreen(false)
    await vi.advanceTimersByTimeAsync(1000)
    expect(requests[0]!.signal?.aborted).toBe(true)
    onScreen(true)
    await vi.advanceTimersByTimeAsync(0)
    expect(requests).toHaveLength(2)

    const visibility = vi.spyOn(document, 'visibilityState', 'get').mockReturnValue('hidden')
    document.dispatchEvent(new Event('visibilitychange'))
    await vi.advanceTimersByTimeAsync(1000)
    expect(requests[1]!.signal?.aborted).toBe(true)
    visibility.mockReturnValue('visible')
    document.dispatchEvent(new Event('visibilitychange'))
    await vi.advanceTimersByTimeAsync(0)
    expect(requests).toHaveLength(3)
  })

  it('flashes when a capture is taken', async () => {
    await show()
    expect(wrapper!.find('.live-image__flash').exists()).toBe(false)
    flashLiveImage()
    await flushPromises()
    expect(wrapper!.find('.live-image__flash').exists()).toBe(true)
  })
})

describe('the View page', () => {
  it('turns the stream off and on, and remembers the choice', async () => {
    connect()
    await show(ViewPage)
    expect(wrapper!.find('h1').text()).toBe('View')
    expect(requests).toHaveLength(1)

    await wrapper!.get('button[aria-label="Disable stream"]').trigger('click')
    await vi.advanceTimersByTimeAsync(1000)
    expect(status()).toBe('disabled')
    expect(requests[0]!.signal?.aborted).toBe(true)
    expect(window.localStorage.getItem(STREAM_DISABLED_KEY)).toBe('true')

    // A new page starts with the choice made.
    wrapper!.unmount()
    setActivePinia(createPinia())
    connect()
    await show(ViewPage)
    expect(status()).toBe('disabled')
    expect(requests).toHaveLength(1)
    await wrapper!.get('.live-image button').trigger('click')
    await vi.advanceTimersByTimeAsync(0)
    expect(requests).toHaveLength(2)
    expect(wrapper!.find('button[aria-label="Disable stream"]').exists()).toBe(true)
    expect(window.localStorage.getItem(STREAM_DISABLED_KEY)).toBe('false')
  })
})
