/**
 * The latest sequence, as the Sequence tab follows it, as the Slide Scan tab
 * follows a scan. Its details are read every second. The sequence shown is
 * the one the tab starts, or one found running however it was started, such
 * as in another window, until Close. A sequence the server refuses before it
 * begins isn't shown: its error is reported instead.
 */
import { computed, onScopeDispose, ref, shallowRef, watch } from 'vue'

import { type LiveSequenceDetails, type TypedInvocation, useThing } from '@/api/things'
import { ApiError } from '@/api/wot/errors'
import type { InvocationRecord } from '@/api/wot/invocation'
import { splitDuration } from '@/server-ui/duration'
import { settled } from '@/server-ui/writes'

/** How often the details are read, in ms. */
const POLL_MS = 1000

/** `seconds` as a clock's hours, minutes and seconds: `1:02:03`. */
export function clock(seconds: number) {
  const { hours, minutes, seconds: rest } = splitDuration(Math.max(0, Math.round(seconds)))
  return [hours, minutes, rest].map((n, i) => (i ? String(n).padStart(2, '0') : n)).join(':')
}

export function useSequence(onRefused: (error: unknown) => void) {
  const sequence = useThing('sequence')
  const details = ref<LiveSequenceDetails | null>(null)
  const shown = shallowRef<TypedInvocation<string>>()
  const record = ref<InvocationRecord>()
  const cancelling = ref(false)
  // Whether the shown sequence has been seen to begin, so its details are its own.
  const began = ref(false)
  // Its name, once it has ended well.
  const ended = ref<string>()
  let timer: ReturnType<typeof setTimeout> | undefined
  // Each run of polling, so a run that's been replaced stops.
  let run = 0

  const onUpdate = (latest: InvocationRecord) => (record.value = latest)

  function show(invocation: TypedInvocation<string>, seenToBegin: boolean) {
    shown.value = invocation
    record.value = invocation.record
    cancelling.value = false
    began.value = seenToBegin
    ended.value = undefined
    invocation.output().then(
      // One too short to be seen running has still begun.
      (name) => ([began.value, ended.value] = [true, name]),
      (error) => {
        if (began.value || !(error instanceof ApiError) || error.kind === 'cancelled') return
        onRefused(error)
        close()
      },
    )
  }

  async function poll(mine: number) {
    const thing = sequence.value
    try {
      if (thing) {
        details.value = await thing.read('latest_sequence_live_details')
        const running = details.value && details.value.result == null
        if (running) began.value = true
        if (running && !shown.value) {
          const [ongoing] = await thing.thing.ongoingInvocations('run_sequence', { onUpdate })
          if (ongoing && !shown.value) show(ongoing as TypedInvocation<string>, true)
        }
      }
    } catch {
      // Tried again next time.
    }
    if (mine === run) timer = setTimeout(() => void poll(mine), POLL_MS)
  }
  // At once, and again as soon as there's a microscope to ask.
  watch(
    sequence,
    () => {
      clearTimeout(timer)
      void poll(++run)
    },
    { immediate: true },
  )
  onScopeDispose(() => clearTimeout(timer))

  /** Starts a sequence called `name`, or by the time if it's empty. */
  async function start(name: string) {
    const thing = sequence.value
    if (!thing || shown.value) return
    details.value = null
    try {
      // The settings just edited, first.
      await settled()
      show(await thing.invoke('run_sequence', { name }, { onUpdate }), false)
    } catch (error) {
      onRefused(error)
    }
  }

  /** Asks the server to cancel the sequence shown. */
  async function cancel() {
    cancelling.value = true
    await shown.value?.cancel().catch(() => (cancelling.value = false))
  }

  /** Stops showing the sequence, once it has ended. */
  function close() {
    shown.value = undefined
    record.value = undefined
  }

  const running = computed(() => ['pending', 'running'].includes(record.value?.status ?? ''))
  const log = computed(() => (record.value?.log ?? []).map((entry) => entry.message))
  /** The shown sequence's details, once they're its own. */
  const current = computed(() => {
    const own = shown.value && began.value && (!ended.value || details.value?.name === ended.value)
    return own ? details.value : null
  })

  return { sequence, shown, current, running, cancelling, log, start, cancel, close }
}
