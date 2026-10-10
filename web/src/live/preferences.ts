import { defineStore } from 'pinia'
import { ref, watch } from 'vue'

/** Where the "Disable stream" preference is kept, in this browser. */
export const STREAM_DISABLED_KEY = 'microscope.stream-disabled'

function readDisabled(): boolean {
  try {
    return window.localStorage.getItem(STREAM_DISABLED_KEY) === 'true'
  } catch {
    return false
  }
}

/** The live image's preferences, kept in this browser. */
export const useLiveImagePreferences = defineStore('live-image', () => {
  /** Whether the live stream is off, to spare the network or the computer. */
  const streamDisabled = ref(readDisabled())
  watch(streamDisabled, (disabled) => {
    try {
      window.localStorage.setItem(STREAM_DISABLED_KEY, String(disabled))
    } catch {
      // Without storage, the choice lasts until the page reloads.
    }
  })
  return { streamDisabled }
})
