import { onBeforeUnmount, onMounted, type Ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import type { Destination } from './navigation'

/** Whether a key press is someone typing into a field, which shortcuts must leave alone. */
function isTyping(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLElement &&
    (target.isContentEditable || ['INPUT', 'SELECT', 'TEXTAREA'].includes(target.tagName))
  )
}

/**
 * Shift+↓ and Shift+↑ go to the next and previous destination in the rail,
 * wrapping around. (P09's shortcut registry will take this over.)
 */
export function useTabCycling(destinations: Ref<readonly Destination[]>) {
  const router = useRouter()
  const route = useRoute()

  function onKeydown(event: KeyboardEvent) {
    if (!event.shiftKey || event.altKey || event.ctrlKey || event.metaKey) return
    if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return
    if (isTyping(event.target)) return
    const list = destinations.value
    if (list.length === 0) return

    const step = event.key === 'ArrowDown' ? 1 : -1
    const current = list.findIndex((destination) => destination.id === route.name)
    const next =
      current === -1
        ? list[step > 0 ? 0 : list.length - 1]
        : list[(current + step + list.length) % list.length]
    event.preventDefault()
    void router.push({ name: next!.id })
  }

  onMounted(() => window.addEventListener('keydown', onKeydown))
  onBeforeUnmount(() => window.removeEventListener('keydown', onKeydown))
}
