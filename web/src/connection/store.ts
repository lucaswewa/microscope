import { defineStore } from 'pinia'
import { ref, shallowRef, watch } from 'vue'

import { WotClient } from '@/api/wot/client'
import type { ThingDescription } from '@/api/wot/td'

import {
  apiRoot,
  locationFor,
  profileFromLocation,
  readRecent,
  withRecent,
  writeRecent,
  type Profile,
} from './profiles'

/**
 * Where the connection stands (ADR-0015):
 * - `idle`: not connected, and not trying;
 * - `connecting`: reaching a server for the first time;
 * - `connected`: the server answers, and has the required Things;
 * - `lost`: it doesn't, or never did; a new attempt is scheduled;
 * - `reconnecting`: trying again.
 */
export type ConnectionState = 'idle' | 'connecting' | 'connected' | 'lost' | 'reconnecting'

/** The Things the app can't work without. The camera joins them in P17. */
export const REQUIRED_THINGS = ['system']
/** How often a connected app checks that the server is still there. */
export const HEARTBEAT_MS = 5000
/** The first wait before trying again; it doubles each time, up to the maximum. */
export const FIRST_RETRY_MS = 1000
export const MAX_RETRY_MS = 10_000

/**
 * The connection to the microscope. The rest of the app uses `client` and
 * the cached `descriptions`, and doesn't need to know where the server is.
 */
export const useConnectionStore = defineStore('connection', () => {
  const state = ref<ConnectionState>('idle')
  const profile = shallowRef<Profile>()
  const client = shallowRef<WotClient>()
  /** The Things' descriptions, kept while the connection is lost. */
  const descriptions = shallowRef<Readonly<Record<string, ThingDescription>>>()
  const hostname = ref<string>()
  /** Why the last attempt, or the connection, failed. */
  const error = shallowRef<unknown>()
  const recent = ref(readRecent())

  // Each connect() starts a new attempt; answers to older ones are ignored.
  let attempt = 0
  let timer: ReturnType<typeof setTimeout> | undefined
  let retryDelay = FIRST_RETRY_MS

  function schedule(task: () => void, ms: number) {
    clearTimeout(timer)
    timer = setTimeout(task, ms)
  }

  /** Connects to a profile's server, leaving any other, and keeps the page's address in step. */
  async function connect(next: Profile) {
    clearTimeout(timer)
    attempt += 1
    profile.value = next
    client.value = new WotClient({ baseUrl: apiRoot(next) })
    descriptions.value = undefined
    hostname.value = undefined
    error.value = undefined
    retryDelay = FIRST_RETRY_MS
    state.value = 'connecting'
    window.history.replaceState(window.history.state, '', locationFor(next, window.location))
    if (next.kind === 'remote') {
      recent.value = withRecent(recent.value, next.origin)
      writeRecent(recent.value)
    }
    await reach(attempt)
  }

  /** Reads the descriptions and the host name: connected if that works, lost if not. */
  async function reach(current: number) {
    const wot = client.value!
    try {
      const things = await wot.thingDescriptions()
      const missing = REQUIRED_THINGS.filter((thing) => !(thing in things))
      if (missing.length > 0) throw new Error(`This server has no ${missing.join(' or ')} Thing.`)
      const name = await wot.consume(things.system!).readProperty<string>('hostname')
      if (current !== attempt) return
      descriptions.value = things
      hostname.value = name
      error.value = undefined
      retryDelay = FIRST_RETRY_MS
      state.value = 'connected'
      schedule(() => void checkStillThere(current), HEARTBEAT_MS)
    } catch (failure) {
      if (current === attempt) lose(failure)
    }
  }

  async function checkStillThere(current: number) {
    try {
      await client.value!.request(new URL('health', apiRoot(profile.value!)))
      if (current === attempt) schedule(() => void checkStillThere(current), HEARTBEAT_MS)
    } catch (failure) {
      if (current === attempt) lose(failure)
    }
  }

  function lose(failure: unknown) {
    error.value = failure
    state.value = 'lost'
    schedule(reconnect, retryDelay)
    retryDelay = Math.min(retryDelay * 2, MAX_RETRY_MS)
  }

  function reconnect() {
    state.value = 'reconnecting'
    void reach(attempt)
  }

  /** Tries again at once, after a failure. */
  function retry() {
    if (state.value !== 'lost') return
    clearTimeout(timer)
    retryDelay = FIRST_RETRY_MS
    reconnect()
  }

  /** Stops trying. The descriptions stay until the next connection. */
  function disconnect() {
    clearTimeout(timer)
    attempt += 1
    state.value = 'idle'
  }

  /** Connects as the page's address asks (`?microscope=`), or to the server that served it. */
  function start() {
    void connect(profileFromLocation(window.location))
  }

  function forget(origin: string) {
    recent.value = recent.value.filter((other) => other !== origin)
    writeRecent(recent.value)
  }

  /** Whether the server has a Thing. Before any description is known, every Thing counts. */
  function isAvailable(thing: string): boolean {
    return descriptions.value === undefined || thing in descriptions.value
  }

  // The window's title names the microscope.
  watch(hostname, (name) => {
    document.title = name ? `${name} – Microscope` : 'Microscope'
  })

  return {
    state,
    profile,
    client,
    descriptions,
    hostname,
    error,
    recent,
    connect,
    retry,
    disconnect,
    start,
    forget,
    isAvailable,
  }
})
