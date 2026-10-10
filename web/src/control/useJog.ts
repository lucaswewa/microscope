import { onBeforeUnmount, onMounted } from 'vue'

import { useThing } from '@/api/things'
import { useShortcut } from '@/app/shortcuts'

import { JogController, type Axes } from './jog'
import { useNavigationPreferences } from './navigation'

/**
 * Directions as the image shows them: ↑ shows what is above, which in the
 * simulator is −y.
 */
export const UP: Axes = { x: 0, y: -1, z: 0 }
export const DOWN: Axes = { x: 0, y: 1, z: 0 }
export const LEFT: Axes = { x: -1, y: 0, z: 0 }
export const RIGHT: Axes = { x: 1, y: 0, z: 0 }
export const FOCUS_UP: Axes = { x: 0, y: 0, z: 1 }
export const FOCUS_DOWN: Axes = { x: 0, y: 0, z: -1 }

const KEYS: [keys: string, direction: Axes, description: string][] = [
  ['ArrowUp', UP, 'Move up'],
  ['ArrowDown', DOWN, 'Move down'],
  ['ArrowLeft', LEFT, 'Move left'],
  ['ArrowRight', RIGHT, 'Move right'],
  ['PageUp', FOCUS_UP, 'Focus up'],
  ['PageDown', FOCUS_DOWN, 'Focus down'],
]

/**
 * A jog controller for the stage, which the arrow keys and PgUp/PgDn drive
 * while the calling component is mounted: a press moves a step, and holding
 * keeps moving. Losing the window's focus releases every key, since their
 * key-ups would be missed.
 */
export function useJog() {
  const stage = useThing('stage')
  const preferences = useNavigationPreferences()
  const ignore = () => {}
  const controller = new JogController({
    jog: (steps) => void stage.value?.invoke('jog', steps).catch(ignore),
    stop: () => void stage.value?.invoke('jog', { stop: true }).catch(ignore),
    steps: preferences.signedSteps,
  })

  useShortcut(
    ...KEYS.map(([keys, direction, description]) => ({
      keys,
      description: `${description}; hold to keep moving`,
      group: 'Stage',
      run: (event: KeyboardEvent) => {
        if (!event.repeat) controller.press(keys, direction)
      },
    })),
  )
  const onKeyup = (event: KeyboardEvent) => controller.release(event.key)
  const releaseAll = () => controller.releaseAll()
  onMounted(() => {
    window.addEventListener('keyup', onKeyup)
    window.addEventListener('blur', releaseAll)
    document.addEventListener('visibilitychange', releaseAll)
  })
  onBeforeUnmount(() => {
    window.removeEventListener('keyup', onKeyup)
    window.removeEventListener('blur', releaseAll)
    document.removeEventListener('visibilitychange', releaseAll)
    controller.releaseAll()
  })
  return controller
}
