/** How often the wheel's turns are sent as one jog, in ms. */
export const WHEEL_INTERVAL_MS = 100

/** Wheel turns ("notches") in an event, whatever unit its delta is in. */
export function notches(event: Pick<WheelEvent, 'deltaY' | 'deltaMode'>): number {
  if (event.deltaMode === 1) return event.deltaY / 3 // lines: three to a notch
  if (event.deltaMode === 2) return event.deltaY // pages
  return event.deltaY / 100 // pixels: a hundred to a notch
}

/**
 * Focusing with the mouse wheel: turning it away from you focuses up, a step
 * a notch. The turns are added up and sent as one jog every `interval`,
 * since each jog replaces the one before it.
 */
export function wheelFocus(
  jog: (z: number) => void,
  step: () => number,
  interval = WHEEL_INTERVAL_MS,
) {
  let turned = 0
  let timer: ReturnType<typeof setTimeout> | undefined

  function send() {
    timer = undefined
    const z = Math.round(-turned * step())
    turned = 0
    if (z !== 0) jog(z)
  }

  return (event: WheelEvent) => {
    event.preventDefault()
    turned += notches(event)
    timer ??= setTimeout(send, interval)
  }
}
