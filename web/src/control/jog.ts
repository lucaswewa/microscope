/** Steps, or a direction (-1, 0 or 1), on each axis. */
export interface Axes {
  x: number
  y: number
  z: number
}

export interface JogOptions {
  /** Starts a jog by these steps, replacing any in progress. */
  jog: (steps: Axes) => void
  /** Stops the stage. */
  stop: () => void
  /** How far one step goes on each axis. */
  steps: () => Axes
  /** How often a held jog is renewed, in ms. */
  interval?: number
  /** How far ahead a held jog aims, in steps: far enough that the stage keeps moving until the next renewal. */
  reach?: number
}

/** How often a held jog is renewed, as in OpenFlexure. */
export const JOG_INTERVAL_MS = 300
/** How far ahead a held jog aims, in steps. */
export const JOG_REACH = 5

/**
 * Turns presses of the d-pad, the focus buttons and the keys into jogs.
 *
 * A press moves one step at once. If it is still held after `interval`, the
 * jog becomes continuous: it is renewed every `interval`, aiming `reach`
 * steps ahead, and releasing stops the stage. A released tap isn't stopped,
 * so its step completes. Presses combine (↑ and → move diagonally), and a
 * press of something already held, such as a key's auto-repeat, is ignored.
 */
export class JogController {
  private readonly held = new Map<string, Axes>()
  private timer?: ReturnType<typeof setInterval>
  private continuous = false

  constructor(private readonly options: JogOptions) {}

  /** `source` (a key, or a button) is pressed, towards `direction`. */
  press(source: string, direction: Axes) {
    if (this.held.has(source)) return
    this.held.set(source, direction)
    const combined = this.direction()
    if (this.continuous) {
      this.send(combined, this.options.reach ?? JOG_REACH)
      return
    }
    this.send(combined, 1)
    this.timer ??= setInterval(() => {
      this.continuous = true
      this.send(this.direction(), this.options.reach ?? JOG_REACH)
    }, this.options.interval ?? JOG_INTERVAL_MS)
  }

  /** `source` is released. */
  release(source: string) {
    if (!this.held.delete(source)) return
    const combined = this.direction()
    if (isStill(combined)) this.end()
    else if (this.continuous) this.send(combined, this.options.reach ?? JOG_REACH)
  }

  /** Everything is released: when the window loses focus, or the page is left. */
  releaseAll() {
    if (this.held.size === 0) return
    this.held.clear()
    this.end()
  }

  private end() {
    clearInterval(this.timer)
    this.timer = undefined
    if (this.continuous) this.options.stop()
    this.continuous = false
  }

  /** The held directions together, each axis kept to -1, 0 or 1. */
  private direction(): Axes {
    const sum = { x: 0, y: 0, z: 0 }
    for (const direction of this.held.values()) {
      sum.x += direction.x
      sum.y += direction.y
      sum.z += direction.z
    }
    return { x: Math.sign(sum.x), y: Math.sign(sum.y), z: Math.sign(sum.z) }
  }

  private send(direction: Axes, multiple: number) {
    if (isStill(direction)) return
    const steps = this.options.steps()
    this.options.jog({
      x: direction.x * steps.x * multiple,
      y: direction.y * steps.y * multiple,
      z: direction.z * steps.z * multiple,
    })
  }
}

function isStill(direction: Axes) {
  return direction.x === 0 && direction.y === 0 && direction.z === 0
}
