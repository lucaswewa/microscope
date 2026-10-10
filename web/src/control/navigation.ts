import { defineStore } from 'pinia'
import { ref, watch } from 'vue'

import type { Axes } from './jog'

/** Where the navigation preferences are kept, in this browser. */
export const NAVIGATION_KEY = 'microscope.navigation'

/** How far one step goes on each axis: OpenFlexure's defaults. */
export const DEFAULT_STEPS: Axes = { x: 200, y: 200, z: 50 }

export interface Inversion {
  x: boolean
  y: boolean
  z: boolean
}

interface Saved {
  steps: Axes
  invert: Inversion
}

const AXES = ['x', 'y', 'z'] as const

/** The saved preferences, keeping the defaults for anything missing or invalid. */
function read(): Saved {
  const saved: Saved = { steps: { ...DEFAULT_STEPS }, invert: { x: false, y: false, z: false } }
  try {
    const parsed = JSON.parse(window.localStorage.getItem(NAVIGATION_KEY) ?? '{}')
    for (const axis of AXES) {
      const step = parsed?.steps?.[axis]
      if (Number.isInteger(step) && step > 0) saved.steps[axis] = step
      if (typeof parsed?.invert?.[axis] === 'boolean') saved.invert[axis] = parsed.invert[axis]
    }
  } catch {
    // Unreadable, or no storage: the defaults.
  }
  return saved
}

/**
 * How the stage is navigated, kept in this browser: how far a step goes on
 * each axis, and which axes move the other way, for set-ups whose camera
 * sees the stage turned or mirrored.
 */
export const useNavigationPreferences = defineStore('navigation', () => {
  const saved = read()
  const steps = ref<Axes>(saved.steps)
  const invert = ref<Inversion>(saved.invert)
  watch(
    [steps, invert],
    () => {
      try {
        const value = { steps: steps.value, invert: invert.value }
        window.localStorage.setItem(NAVIGATION_KEY, JSON.stringify(value))
      } catch {
        // Without storage, the preferences last until the page reloads.
      }
    },
    { deep: true },
  )

  /** A step on each axis, signed by the inversion: what a press towards + moves. */
  function signedSteps(): Axes {
    const sign = (axis: (typeof AXES)[number]) => (invert.value[axis] ? -1 : 1)
    return {
      x: steps.value.x * sign('x'),
      y: steps.value.y * sign('y'),
      z: steps.value.z * sign('z'),
    }
  }

  return { steps, invert, signedSteps }
})
