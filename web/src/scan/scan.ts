/**
 * The latest scan, as the Slide Scan tab follows it (ADR-0029). Its details
 * are read every second. The scan shown is the one the tab starts, or one
 * found running however it was started, such as in another window, until
 * Close. A scan the server refuses before it begins isn't shown: its error
 * is reported instead.
 */
import { computed, onScopeDispose, ref, shallowRef, watch } from 'vue'

import { type LiveScanDetails, type TypedInvocation, useThing } from '@/api/things'
import { ApiError } from '@/api/wot/errors'
import type { InvocationRecord } from '@/api/wot/invocation'
import { settled } from '@/server-ui/writes'

/** How often the details are read, in ms. */
const POLL_MS = 1000

export function useScan(onRefused: (error: unknown) => void) {
  const smartScan = useThing('smart_scan')
  const details = ref<LiveScanDetails | null>(null)
  const shown = shallowRef<TypedInvocation<string>>()
  const record = ref<InvocationRecord>()
  const cancelling = ref(false)
  // Whether the shown scan has been seen to begin, so its details are its own.
  const began = ref(false)
  let timer: ReturnType<typeof setTimeout> | undefined
  // Each run of polling, so a run that's been replaced stops.
  let run = 0

  const onUpdate = (latest: InvocationRecord) => (record.value = latest)

  function show(invocation: TypedInvocation<string>, seenToBegin: boolean) {
    shown.value = invocation
    record.value = invocation.record
    cancelling.value = false
    began.value = seenToBegin
    invocation.output().catch((error) => {
      if (began.value || !(error instanceof ApiError) || error.kind === 'cancelled') return
      onRefused(error)
      close()
    })
  }

  async function poll(mine: number) {
    const thing = smartScan.value
    try {
      if (thing) {
        details.value = await thing.read('latest_scan_live_details')
        const scanning = details.value && details.value.scan_phase !== 'complete'
        if (scanning) began.value = true
        if (scanning && !shown.value) {
          const [ongoing] = await thing.thing.ongoingInvocations('sample_scan', { onUpdate })
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
    smartScan,
    () => {
      clearTimeout(timer)
      void poll(++run)
    },
    { immediate: true },
  )
  onScopeDispose(() => clearTimeout(timer))

  /** Starts a scan called `scanName`, or by the time if it's empty. */
  async function start(scanName: string) {
    const thing = smartScan.value
    if (!thing || shown.value) return
    details.value = null
    try {
      // The settings just edited, first.
      await settled()
      show(await thing.invoke('sample_scan', { scan_name: scanName }, { onUpdate }), false)
    } catch (error) {
      onRefused(error)
    }
  }

  /** Asks the server to cancel the scan shown. */
  async function cancel() {
    cancelling.value = true
    await shown.value?.cancel().catch(() => (cancelling.value = false))
  }

  /** Stops showing the scan, once it has ended. */
  function close() {
    shown.value = undefined
    record.value = undefined
  }

  const running = computed(() => ['pending', 'running'].includes(record.value?.status ?? ''))
  const log = computed(() => (record.value?.log ?? []).map((entry) => entry.message))
  /** The shown scan's details, once they're its own. */
  const current = computed(() => (shown.value && began.value ? details.value : null))

  return { smartScan, shown, record, current, running, cancelling, log, start, cancel, close }
}
