/**
 * Where the app connects (ADR-0015): the server that served the page, or
 * another microscope by its address.
 */
export type Profile = { kind: 'local' } | { kind: 'remote'; origin: string }

/** The API root of a profile's server, under ADR-0007's prefix. */
export function apiRoot(profile: Profile): URL {
  const origin = profile.kind === 'local' ? window.location.origin : profile.origin
  return new URL('/api/v1/', origin)
}

/** An origin as people write it: `lab-pc:5000`, keeping `https://` but not `http://`. */
export function displayOrigin(origin: string): string {
  return origin.replace(/^http:\/\//, '')
}

/**
 * The origin of a server address as someone types it: `lab-pc:5000`,
 * `http://192.168.1.20:5000` or `https://scope.lab/api/v1/`. Without a
 * scheme it is http; any path is dropped. Throws an Error that says what's
 * wrong.
 */
export function parseAddress(text: string): string {
  const address = text.trim()
  if (!address) throw new Error('Enter the microscope’s address, such as lab-pc:5000.')
  let url: URL
  try {
    url = new URL(/^[a-z][a-z\d+.-]*:\/\//i.test(address) ? address : `http://${address}`)
  } catch {
    throw new Error(`“${address}” isn’t an address.`)
  }
  if (url.protocol !== 'http:' && url.protocol !== 'https:') {
    throw new Error('The address must start with http:// or https://, or with neither.')
  }
  return url.origin
}

/** The profile the page's address asks for: `?microscope=<address>`, or else this server. */
export function profileFromLocation(location: Location): Profile {
  const address = new URLSearchParams(location.search).get('microscope')
  if (address === null) return { kind: 'local' }
  try {
    return { kind: 'remote', origin: parseAddress(address) }
  } catch {
    return { kind: 'local' }
  }
}

/** The page's address for a profile, so that reloading reconnects the same way. */
export function locationFor(profile: Profile, location: Location): string {
  const url = new URL(location.href)
  if (profile.kind === 'remote') url.searchParams.set('microscope', displayOrigin(profile.origin))
  else url.searchParams.delete('microscope')
  return url.href
}

export const RECENT_STORAGE_KEY = 'microscope.recentConnections'
const MAX_RECENT = 5

/** The remote origins connected to lately, most recent first. Empty if storage fails. */
export function readRecent(): string[] {
  try {
    const saved: unknown = JSON.parse(window.localStorage.getItem(RECENT_STORAGE_KEY) ?? '[]')
    return Array.isArray(saved)
      ? saved.filter((origin): origin is string => typeof origin === 'string').slice(0, MAX_RECENT)
      : []
  } catch {
    return []
  }
}

/** Saves the recent origins, if storage allows. */
export function writeRecent(origins: readonly string[]): void {
  try {
    window.localStorage.setItem(RECENT_STORAGE_KEY, JSON.stringify(origins))
  } catch {
    // Storage is unavailable (blocked, or private browsing): they're forgotten.
  }
}

/** `origins` with `origin` first, without repeats, at most five. */
export function withRecent(origins: readonly string[], origin: string): string[] {
  return [origin, ...origins.filter((other) => other !== origin)].slice(0, MAX_RECENT)
}
