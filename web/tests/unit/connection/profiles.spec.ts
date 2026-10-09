import { afterEach, describe, expect, it, vi } from 'vitest'

import {
  RECENT_STORAGE_KEY,
  locationFor,
  parseAddress,
  profileFromLocation,
  readRecent,
  withRecent,
  writeRecent,
} from '@/connection/profiles'

const at = (href: string) => new URL(href) as unknown as Location

afterEach(() => {
  window.localStorage.clear()
  vi.restoreAllMocks()
})

describe('parseAddress', () => {
  it('reads addresses as people type them', () => {
    expect(parseAddress('lab-pc:5000')).toBe('http://lab-pc:5000')
    expect(parseAddress(' http://192.168.1.20:5000 ')).toBe('http://192.168.1.20:5000')
    expect(parseAddress('https://scope.lab/api/v1/')).toBe('https://scope.lab')
    expect(parseAddress('HTTP://Lab-PC')).toBe('http://lab-pc')
  })

  it('says what’s wrong with an address it can’t use', () => {
    expect(() => parseAddress('  ')).toThrow('Enter the microscope’s address')
    expect(() => parseAddress('http://')).toThrow('“http://” isn’t an address.')
    expect(() => parseAddress('ftp://lab-pc')).toThrow('must start with http:// or https://')
  })
})

describe('the page’s address', () => {
  it('asks for a remote microscope with ?microscope=, or else this server', () => {
    expect(profileFromLocation(at('http://app/?microscope=lab-pc:5000#/view'))).toEqual({
      kind: 'remote',
      origin: 'http://lab-pc:5000',
    })
    expect(profileFromLocation(at('http://app/#/view'))).toEqual({ kind: 'local' })
    expect(profileFromLocation(at('http://app/?microscope=ftp://x#/'))).toEqual({ kind: 'local' })
  })

  it('follows the profile, keeping the route', () => {
    const remote = { kind: 'remote', origin: 'http://lab-pc:5000' } as const
    const href = locationFor(remote, at('http://app/#/control'))
    expect(profileFromLocation(at(href))).toEqual(remote)
    expect(new URL(href).hash).toBe('#/control')
    expect(locationFor({ kind: 'local' }, at(href))).toBe('http://app/#/control')
  })
})

describe('recent connections', () => {
  it('keeps five, most recent first, without repeats', () => {
    let recent: string[] = []
    for (const n of [1, 2, 3, 2, 4, 5, 6]) recent = withRecent(recent, `http://pc${n}`)
    expect(recent).toEqual(['http://pc6', 'http://pc5', 'http://pc4', 'http://pc2', 'http://pc3'])
  })

  it('are saved, and read back without anything that isn’t an origin', () => {
    writeRecent(['http://pc1'])
    expect(readRecent()).toEqual(['http://pc1'])
    window.localStorage.setItem(RECENT_STORAGE_KEY, '["http://pc2", 3, null]')
    expect(readRecent()).toEqual(['http://pc2'])
    window.localStorage.setItem(RECENT_STORAGE_KEY, 'not json')
    expect(readRecent()).toEqual([])
  })

  it('are forgotten, quietly, when storage fails', () => {
    vi.spyOn(window.localStorage, 'getItem').mockImplementation(() => {
      throw new Error('blocked')
    })
    vi.spyOn(window.localStorage, 'setItem').mockImplementation(() => {
      throw new Error('blocked')
    })
    expect(() => writeRecent(['http://pc1'])).not.toThrow()
    expect(readRecent()).toEqual([])
  })
})
