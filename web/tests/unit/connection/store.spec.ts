import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { useConnectionStore, type ConnectionState } from '@/connection/store'
import type { ThingDescription } from '@/api/wot/td'

import { fakeServer, json, type Received } from '../api/fakeServer'

const system: ThingDescription = {
  title: 'MicroscopeSystem',
  properties: {
    hostname: {
      type: 'string',
      forms: [{ href: '/api/v1/system/hostname', op: ['readproperty'] }],
    },
  },
}

/** A server with the system Thing, which goes away and comes back when told. */
function server(things: Record<string, ThingDescription> = { system }) {
  const control = { down: false }
  const answer = (body: unknown) => () =>
    control.down ? Promise.reject(new TypeError('Failed to fetch')) : json(body)
  const fake = fakeServer({
    'GET /api/v1/thing_descriptions/': answer(things),
    'GET /api/v1/system/hostname': answer('lab-pc'),
    'GET /api/v1/health': answer({ status: 'ok' }),
  })
  vi.stubGlobal('fetch', fake.fetch)
  return { control, received: fake.received }
}

let states: ConnectionState[]

beforeEach(() => {
  vi.useFakeTimers()
  window.history.replaceState(null, '', '/')
  window.localStorage.clear()
  setActivePinia(createPinia())
  states = []
  const connection = useConnectionStore()
  connection.$subscribe(() => {
    if (states.at(-1) !== connection.state) states.push(connection.state)
  })
})

afterEach(() => {
  vi.useRealTimers()
  vi.unstubAllGlobals()
})

describe('the connection', () => {
  it('connects to this server: descriptions, host name and window title', async () => {
    server()
    const connection = useConnectionStore()
    await connection.connect({ kind: 'local' })
    expect(states).toEqual(['connecting', 'connected'])
    expect(Object.keys(connection.descriptions!)).toEqual(['system'])
    expect(connection.hostname).toBe('lab-pc')
    await vi.advanceTimersByTimeAsync(0)
    expect(document.title).toBe('lab-pc – Microscope')
  })

  it('is lost when the server can’t be reached, and tries again, waiting longer each time', async () => {
    const { control, received } = server()
    control.down = true
    const connection = useConnectionStore()
    await connection.connect({ kind: 'local' })
    expect(states).toEqual(['connecting', 'lost'])
    expect(connection.error).toMatchObject({ kind: 'network' })
    await vi.advanceTimersByTimeAsync(1000)
    await vi.advanceTimersByTimeAsync(1999)
    expect(received).toHaveLength(2) // the next attempt is 2 s after the last
    control.down = false
    await vi.advanceTimersByTimeAsync(1)
    expect(states).toEqual([
      'connecting',
      'lost',
      'reconnecting',
      'lost',
      'reconnecting',
      'connected',
    ])
  })

  it('notices when a connected server goes, keeps its Things, and recovers', async () => {
    const { control } = server()
    const connection = useConnectionStore()
    await connection.connect({ kind: 'local' })
    control.down = true
    await vi.advanceTimersByTimeAsync(5000) // the heartbeat
    expect(connection.state).toBe('lost')
    expect(connection.descriptions).toBeDefined()
    expect(connection.isAvailable('gallery')).toBe(false)
    control.down = false
    await vi.advanceTimersByTimeAsync(1000)
    expect(connection.state).toBe('connected')
  })

  it('retries at once on Retry', async () => {
    const { control } = server()
    control.down = true
    const connection = useConnectionStore()
    await connection.connect({ kind: 'local' })
    control.down = false
    connection.retry()
    await vi.advanceTimersByTimeAsync(0)
    expect(connection.state).toBe('connected')
  })

  it('is lost when the server lacks a required Thing', async () => {
    server({ camera: { title: 'Camera' } })
    const connection = useConnectionStore()
    await connection.connect({ kind: 'local' })
    expect(connection.state).toBe('lost')
    expect(String(connection.error)).toContain('This server has no system Thing.')
  })

  it('ignores the answers to an attempt it has left', async () => {
    const old = (request: Received) => request.url.origin === 'http://pc1:5000'
    const fake = fakeServer({
      // The first microscope answers late, after the app has moved on.
      'GET /api/v1/thing_descriptions/': (request: Received) =>
        old(request)
          ? new Promise<Response>((resolve) => setTimeout(() => resolve(json({ system })), 1000))
          : json({ system }),
      'GET /api/v1/system/hostname': (request: Received) =>
        json(old(request) ? 'old-pc' : 'lab-pc'),
    })
    vi.stubGlobal('fetch', fake.fetch)
    const connection = useConnectionStore()
    const first = connection.connect({ kind: 'remote', origin: 'http://pc1:5000' })
    await connection.connect({ kind: 'local' })
    await vi.advanceTimersByTimeAsync(1000)
    await first
    expect(connection.state).toBe('connected')
    expect(connection.hostname).toBe('lab-pc')
    expect(connection.profile).toEqual({ kind: 'local' })
  })

  it('remembers remote microscopes, and puts them in the page’s address', async () => {
    const { received } = server()
    const connection = useConnectionStore()
    await connection.connect({ kind: 'remote', origin: 'http://lab-pc:5000' })
    expect(received[0]!.url.href).toBe('http://lab-pc:5000/api/v1/thing_descriptions/')
    expect(connection.recent).toEqual(['http://lab-pc:5000'])
    expect(new URL(window.location.href).searchParams.get('microscope')).toBe('lab-pc:5000')
    connection.forget('http://lab-pc:5000')
    expect(connection.recent).toEqual([])
  })

  it('starts as the page’s address asks', async () => {
    const { received } = server()
    window.history.replaceState(null, '', '/?microscope=lab-pc:5000#/view')
    useConnectionStore().start()
    await vi.advanceTimersByTimeAsync(0)
    expect(received[0]!.url.origin).toBe('http://lab-pc:5000')
  })

  it('counts every Thing as available until it knows the server’s', async () => {
    server()
    const connection = useConnectionStore()
    expect(connection.isAvailable('gallery')).toBe(true)
    await connection.connect({ kind: 'local' })
    expect(connection.isAvailable('system')).toBe(true)
    expect(connection.isAvailable('gallery')).toBe(false)
  })

  it('stops trying when disconnected', async () => {
    const { control, received } = server()
    control.down = true
    const connection = useConnectionStore()
    await connection.connect({ kind: 'local' })
    connection.disconnect()
    await vi.advanceTimersByTimeAsync(60_000)
    expect(connection.state).toBe('idle')
    expect(received).toHaveLength(1)
  })
})
