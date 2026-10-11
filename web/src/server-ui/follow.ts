import type { ConsumedThing } from '@/api/wot/client'
import { formUrl } from '@/api/wot/td'

interface Follower {
  name: string
  onValue: (value: unknown) => void
}

/** The shared streams, by their URL. */
const streams = new Map<string, { followers: Set<Follower>; stop: () => void }>()

/**
 * Follows the property `name` of `thing`, calling `onValue` with each new
 * value, and returns a function that stops. Every follower of one Thing
 * shares its single stream of all its observable properties: a browser opens
 * only six connections to a server at a time over HTTP/1.1, and a stream for
 * each property would soon use them up. The stream stops when its last
 * follower does.
 */
export function followProperty(
  thing: ConsumedThing,
  name: string,
  onValue: (value: unknown) => void,
): () => void {
  // The base only tells the Things apart, if the description has none.
  const base = thing.td.base ?? 'http://thing.invalid/'
  const key = formUrl(thing.td.forms, 'observeallproperties', base, { sse: true })?.href
  if (!key) return () => {}
  let stream = streams.get(key)
  if (!stream) {
    const followers = new Set<Follower>()
    const stop = thing.observeProperties((changed, value) => {
      for (const follower of followers) if (follower.name === changed) follower.onValue(value)
    })
    stream = { followers, stop }
    streams.set(key, stream)
  }
  const follower = { name, onValue }
  const shared = stream
  shared.followers.add(follower)
  return () => {
    shared.followers.delete(follower)
    if (shared.followers.size > 0) return
    shared.stop()
    streams.delete(key)
  }
}
