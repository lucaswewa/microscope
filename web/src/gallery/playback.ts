// Playing a sequence in the viewer (P52). Each image shows for `seconds`
// from when it's drawn, so a slow connection slows the playback rather than
// skipping images or showing an empty view. At the end it stops, or starts
// again from the first image if `loop` is set.
import { onScopeDispose, ref, type Ref } from 'vue'

/**
 * Playback of `count` images, moving `page` on. The view calls `shown` once
 * it has drawn a page.
 */
export function usePlayback(page: Ref<number>, count: () => number) {
  const playing = ref(false)
  /** How long each image shows, in seconds. */
  const seconds = ref(0.5)
  const loop = ref(false)
  let timer: ReturnType<typeof setTimeout> | undefined

  function wait() {
    clearTimeout(timer)
    timer = undefined
  }

  /** `drawn` has been drawn: the next follows after `seconds`, while playing. */
  function shown(drawn: number) {
    wait()
    if (!playing.value) return
    timer = setTimeout(() => {
      // Moved on meanwhile: its page says when it's drawn.
      if (drawn !== page.value) return
      if (drawn + 1 < count()) page.value = drawn + 1
      else if (loop.value) page.value = 0
      else playing.value = false
    }, seconds.value * 1000)
  }

  /** Plays from the image shown, or from the first if the last is shown. */
  function play() {
    playing.value = true
    if (page.value === count() - 1 && !loop.value) page.value = 0
    else shown(page.value)
  }

  function pause() {
    playing.value = false
    wait()
  }

  onScopeDispose(wait)
  return { playing, seconds, loop, play, pause, shown }
}
