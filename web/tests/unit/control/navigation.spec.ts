import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { nextTick } from 'vue'

import { NAVIGATION_KEY, useNavigationPreferences } from '@/control/navigation'
import { notches, wheelFocus } from '@/control/wheel'

describe('the navigation preferences', () => {
  beforeEach(() => {
    window.localStorage.clear()
    setActivePinia(createPinia())
  })

  it('start from OpenFlexure’s step sizes, and are kept in this browser', async () => {
    const preferences = useNavigationPreferences()
    expect(preferences.signedSteps()).toEqual({ x: 200, y: 200, z: 50 })
    preferences.steps.z = 20
    preferences.invert.y = true
    await nextTick()
    expect(preferences.signedSteps()).toEqual({ x: 200, y: -200, z: 20 })

    setActivePinia(createPinia())
    expect(useNavigationPreferences().signedSteps()).toEqual({ x: 200, y: -200, z: 20 })
  })

  it('ignore saved values that make no sense', () => {
    const saved = { steps: { x: -5, y: 1.5, z: 30 }, invert: { x: 'yes', z: true } }
    window.localStorage.setItem(NAVIGATION_KEY, JSON.stringify(saved))
    expect(useNavigationPreferences().signedSteps()).toEqual({ x: 200, y: 200, z: -30 })
    window.localStorage.setItem(NAVIGATION_KEY, 'not JSON')
    setActivePinia(createPinia())
    expect(useNavigationPreferences().signedSteps()).toEqual({ x: 200, y: 200, z: 50 })
  })
})

describe('wheel focus', () => {
  beforeEach(() => vi.useFakeTimers())
  afterEach(() => vi.useRealTimers())

  const wheel = (deltaY: number, deltaMode = 0) =>
    new WheelEvent('wheel', { deltaY, deltaMode, cancelable: true })

  it('counts notches in pixels, lines or pages', () => {
    expect(notches(wheel(100))).toBe(1)
    expect(notches(wheel(-6, 1))).toBe(-2)
    expect(notches(wheel(1, 2))).toBe(1)
  })

  it('focuses up a step a notch away from you, sending the turns together', () => {
    const jogs: number[] = []
    const onWheel = wheelFocus(
      (z) => jogs.push(z),
      () => 50,
    )
    const event = wheel(-100)
    onWheel(event)
    onWheel(wheel(-200))
    expect(event.defaultPrevented).toBe(true)
    expect(jogs).toEqual([])
    vi.advanceTimersByTime(100)
    expect(jogs).toEqual([150])

    onWheel(wheel(100))
    onWheel(wheel(-100)) // turned back: nothing to send
    vi.advanceTimersByTime(100)
    expect(jogs).toEqual([150])
  })
})
