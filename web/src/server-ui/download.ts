// Downloads that a Thing's panel (ADR-0024) or a gallery card (ADR-0026)
// describes: what one of a Thing's endpoints answers, the file an action
// returns, or a file in the data folder. Each is saved through the host,
// under the name given for it, and a failure is reported.
import { ApiError } from '@/api/wot/errors'
import { useConnectionStore } from '@/connection/store'
import { useHost } from '@/host'
import { useToast } from '@/ui/overlays'

import type { DownloadButtonElement } from './types'

/** Where the data folder's file at `path` is served, from the API's `base`. */
export function dataUrl(path: string, base: URL): URL {
  return new URL(`data/${path.split('/').map(encodeURIComponent).join('/')}`, base)
}

/** Downloading what buttons describe. */
export function useDownload() {
  const connection = useConnectionStore()
  const host = useHost()
  const toast = useToast()

  /** Why `button`'s file can't be had, if it can't. */
  function missing(button: DownloadButtonElement): string | undefined {
    if (button.source === 'data')
      return button.broken ? `the data folder has no file “${button.path}”` : undefined
    const td = connection.descriptions?.[button.thing]
    const lacksAction = button.source === 'action' && !td?.actions?.[button.action]
    if (!button.broken && td && !lacksAction) return undefined
    return button.source === 'endpoint'
      ? `${button.thing} has no file “${button.path}”`
      : `${button.thing} has no action “${button.action}”`
  }

  /** Where `button`'s file is: the endpoint, the data folder, or the link the action returns. */
  async function locate(button: DownloadButtonElement): Promise<URL> {
    const client = connection.client!
    if (button.source === 'data') return dataUrl(button.path, client.baseUrl)
    if (button.source === 'endpoint')
      return new URL(`${button.thing}/${button.path}`, client.baseUrl)
    const thing = client.consume(connection.descriptions![button.thing]!)
    const output = await (await thing.invokeAction(button.action, button.input ?? {})).output()
    const href = (output as { href?: unknown } | null)?.href
    if (typeof href !== 'string') throw new Error(`${button.action} didn't return a file.`)
    return new URL(href, client.baseUrl)
  }

  /** Saves `button`'s file, or says why it couldn't. */
  async function save(button: DownloadButtonElement): Promise<void> {
    const client = connection.client
    if (!client || missing(button)) return
    try {
      await host.saveFile(button.filename, await client.download(await locate(button)))
    } catch (error) {
      toast.error(`Couldn't download ${button.filename}.`, {
        details: error instanceof ApiError ? (error.body ?? error.message) : error,
      })
    }
  }

  return { missing, save }
}
