import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { effectScope, nextTick, ref, watch } from 'vue'

import { usePlayback } from '@/gallery/playback'

let scope: ReturnType<typeof effectScope>

beforeEach(() => {
  vi.useFakeTimers()
  scope = effectScope()
})

afterEach(() => {
  scope.stop()
  vi.useRealTimers()
})

/**
 * Playback of `count` images from `start`, with a view that draws each page
 * `load` milliseconds after it's asked for.
 */
function playback(count: number, start = 0, load = 0) {
  const page = ref(start)
  const drawn: number[] = []
  const controls = scope.run(() => {
    const controls = usePlayback(page, () => count)
    watch(page, (asked) =>
      setTimeout(() => {
        drawn.push(asked)
        controls.shown(asked)
      }, load),
    )
    return controls
  })!
  return { page, drawn, ...controls }
}

describe('playing a sequence', () => {
  it('shows each image for its seconds, from when it is drawn, and stops at the end', async () => {
    const { page, drawn, playing, seconds, play } = playback(3, 0, 300)
    seconds.value = 1
    play()
    expect(playing.value).toBe(true)
    await vi.advanceTimersByTimeAsync(999)
    expect(page.value).toBe(0)
    await vi.advanceTimersByTimeAsync(1)
    expect(page.value).toBe(1)
    // Its next second starts once it's drawn, 300 ms later.
    await vi.advanceTimersByTimeAsync(1299)
    expect(page.value).toBe(1)
    await vi.advanceTimersByTimeAsync(1)
    expect(page.value).toBe(2)
    await vi.advanceTimersByTimeAsync(5000)
    expect([page.value, playing.value, drawn]).toEqual([2, false, [1, 2]])
  })

  it('starts again from the first image when the last is shown, or loops', async () => {
    const { page, playing, loop, play } = playback(3, 2)
    play()
    await nextTick()
    expect(page.value).toBe(0)
    await vi.advanceTimersByTimeAsync(2500)
    expect([page.value, playing.value]).toEqual([2, false])

    loop.value = true
    play()
    await vi.advanceTimersByTimeAsync(500)
    expect([page.value, playing.value]).toEqual([0, true])
  })

  it('pauses, and follows the image someone moves to', async () => {
    const { page, playing, play, pause } = playback(5)
    play()
    await vi.advanceTimersByTimeAsync(500)
    expect(page.value).toBe(1)
    pause()
    await vi.advanceTimersByTimeAsync(5000)
    expect([page.value, playing.value]).toEqual([1, false])

    play()
    await vi.advanceTimersByTimeAsync(200)
    page.value = 3
    await vi.advanceTimersByTimeAsync(499)
    expect(page.value).toBe(3)
    await vi.advanceTimersByTimeAsync(1)
    expect(page.value).toBe(4)
  })
})
