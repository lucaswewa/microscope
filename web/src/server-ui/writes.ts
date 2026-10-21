// Property writes on their way. An action that reads the settings, such as
// starting a sequence or a scan, waits for them first: the server may run
// it before a write sent a moment earlier, and use the setting from before.

const pending = new Set<Promise<unknown>>()

/** Keeps track of `write` until it's done, and returns it. */
export function track<T>(write: Promise<T>): Promise<T> {
  pending.add(write)
  const done = () => void pending.delete(write)
  write.then(done, done)
  return write
}

/** Completes once every write tracked so far, and any they lead to, is done. */
export async function settled(): Promise<void> {
  while (pending.size > 0) await Promise.allSettled([...pending])
}
