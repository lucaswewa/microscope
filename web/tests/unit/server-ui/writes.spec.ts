import { describe, expect, it } from 'vitest'

import { settled, track } from '@/server-ui/writes'

describe('writes', () => {
  it('settles once every tracked write is done, refused ones too', async () => {
    let finish: () => void = () => {}
    const first = track(new Promise<void>((resolve) => (finish = resolve)))
    const refused = track(Promise.reject(new Error('refused')))
    refused.catch(() => {})
    let done = false
    const waiting = settled().then(() => (done = true))
    await Promise.resolve()
    expect(done).toBe(false)
    // One started while it waits is waited for too.
    let later: () => void = () => {}
    track(new Promise<void>((resolve) => (later = resolve)))
    finish()
    await first
    await new Promise((resolve) => setTimeout(resolve))
    expect(done).toBe(false)
    later()
    await waiting
    expect(done).toBe(true)
    await settled()
  })
})
