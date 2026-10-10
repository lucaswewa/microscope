// @vitest-environment node
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { JogController, type Axes } from '@/control/jog'

beforeEach(() => vi.useFakeTimers())
afterEach(() => vi.useRealTimers())

const UP: Axes = { x: 0, y: -1, z: 0 }
const RIGHT: Axes = { x: 1, y: 0, z: 0 }

/** A controller with steps of 200, 200 and 50, and what it sends. */
function controller() {
  const sent: (Axes | 'stop')[] = []
  const jog = new JogController({
    jog: (steps) => sent.push(steps),
    stop: () => sent.push('stop'),
    steps: () => ({ x: 200, y: 200, z: 50 }),
  })
  return { jog, sent }
}

describe('the jog controller', () => {
  it('moves one step for a tap, and lets it finish', () => {
    const { jog, sent } = controller()
    jog.press('ArrowUp', UP)
    vi.advanceTimersByTime(100)
    jog.release('ArrowUp')
    vi.advanceTimersByTime(1000)
    expect(sent).toEqual([{ x: 0, y: -200, z: 0 }])
  })

  it('keeps moving while held, renewing the jog, and stops on release', () => {
    const { jog, sent } = controller()
    jog.press('pointer:focus-up', { x: 0, y: 0, z: 1 })
    vi.advanceTimersByTime(950)
    jog.release('pointer:focus-up')
    vi.advanceTimersByTime(1000)
    const ahead = { x: 0, y: 0, z: 250 }
    expect(sent).toEqual([{ x: 0, y: 0, z: 50 }, ahead, ahead, ahead, 'stop'])
  })

  it('combines presses, and follows what is still held', () => {
    const { jog, sent } = controller()
    jog.press('ArrowUp', UP)
    jog.press('ArrowRight', RIGHT)
    vi.advanceTimersByTime(300)
    jog.release('ArrowUp')
    jog.release('ArrowRight')
    expect(sent).toEqual([
      { x: 0, y: -200, z: 0 },
      { x: 200, y: -200, z: 0 },
      { x: 1000, y: -1000, z: 0 },
      { x: 1000, y: 0, z: 0 },
      'stop',
    ])
  })

  it('re-aims at once when a press joins a held jog, and never doubles a direction', () => {
    const { jog, sent } = controller()
    jog.press('ArrowRight', RIGHT)
    vi.advanceTimersByTime(300)
    jog.press('pointer:right', RIGHT) // the same way, from the d-pad
    jog.press('ArrowUp', UP)
    expect(sent).toEqual([
      { x: 200, y: 0, z: 0 },
      { x: 1000, y: 0, z: 0 },
      { x: 1000, y: 0, z: 0 },
      { x: 1000, y: -1000, z: 0 },
    ])
  })

  it('ignores a held key’s repeats, and opposite presses cancel out', () => {
    const { jog, sent } = controller()
    jog.press('ArrowRight', RIGHT)
    jog.press('ArrowRight', RIGHT)
    jog.press('ArrowLeft', { x: -1, y: 0, z: 0 })
    jog.release('ArrowLeft')
    jog.release('ArrowRight')
    jog.release('ArrowDown') // never pressed
    vi.advanceTimersByTime(1000)
    expect(sent).toEqual([{ x: 200, y: 0, z: 0 }])
  })

  it('releases everything at once, as when the window loses focus', () => {
    const { jog, sent } = controller()
    jog.press('ArrowUp', UP)
    jog.press('PageDown', { x: 0, y: 0, z: -1 })
    vi.advanceTimersByTime(300)
    jog.releaseAll()
    jog.releaseAll()
    vi.advanceTimersByTime(1000)
    expect(sent.at(-1)).toBe('stop')
    expect(sent.filter((jogged) => jogged === 'stop')).toHaveLength(1)
  })
})
