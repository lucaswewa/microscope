import { beforeAll, describe, expect, it } from 'vitest'

import { WotClient, type ConsumedThing } from '@/api/wot/client'
import { ApiError } from '@/api/wot/errors'
import type { ThingDescription } from '@/api/wot/td'

// The WoT client against a running microscope server, through its `system`
// Thing: the descriptions, property reads, and the server's own errors.
const client = new WotClient({
  baseUrl: process.env.MICROSCOPE_API_URL ?? 'http://127.0.0.1:5000/api/v1/',
})

describe('the WoT client against a running server', () => {
  let descriptions: Record<string, ThingDescription>
  let system: ConsumedThing

  beforeAll(async () => {
    descriptions = await client.thingDescriptions()
    system = client.consume(descriptions.system!)
  })

  it('lists the system Thing’s description, with forms resolved against its base', () => {
    const td = descriptions.system!
    expect(td.title).toBe('MicroscopeSystem')
    expect(Object.keys(td.properties ?? {})).toEqual(
      expect.arrayContaining(['hostname', 'os_version', 'version_data']),
    )
    expect(new URL(td.base!).origin).toBe(client.baseUrl.origin)
  })

  it('reads its properties', async () => {
    expect(await system.readProperty('hostname')).toMatch(/\S/)
    expect(await system.readProperty('version_data')).toEqual({
      version: expect.stringMatching(/^\d+\.\d+\.\d+/),
      version_source: expect.any(String),
    })
  })

  it('reports the server’s errors as ApiErrors', async () => {
    const hostname = new URL('system/hostname', client.baseUrl)
    const readOnly = await client.request(hostname, { method: 'PUT', body: 'x' }).catch((e) => e)
    expect(readOnly).toBeInstanceOf(ApiError)
    expect(readOnly).toMatchObject({ kind: 'http', status: 405, message: 'Method Not Allowed' })
    const missing = await client.request(new URL('system/nothing', client.baseUrl)).catch((e) => e)
    expect(missing).toMatchObject({ kind: 'http', status: 404, message: 'Not Found' })
  })
})
