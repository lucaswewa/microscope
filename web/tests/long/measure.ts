import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'

import type { APIRequestContext, Page, TestInfo } from '@playwright/test'

import { run } from '../servers'

/**
 * Measurements for the long tests (P48): the preview's frame rate, a scan's
 * time per tile, how quickly the app changes tabs, and a process's memory
 * and handles.
 */

/** Whole frames a second from the MJPEG stream at `url`, over `ms`, after a second to start. */
export async function framesPerSecond(url: string, ms = 5000): Promise<number> {
  const response = await fetch(url)
  const boundary = /boundary=([^;]+)/.exec(response.headers.get('content-type') ?? '')![1]!
  const reader = response.body!.getReader()
  const decoder = new TextDecoder('latin1')
  const marker = `--${boundary}`
  let tail = ''
  let frames = 0
  let started = 0
  for (;;) {
    const { value, done } = await reader.read()
    if (done) break
    const text = tail + decoder.decode(value)
    const now = performance.now()
    if (!started) started = now + 1000
    if (now >= started) frames += text.split(marker).length - 1
    tail = text.slice(-marker.length)
    if (now >= started + ms) break
  }
  await reader.cancel()
  return frames / (ms / 1000)
}

/** Runs a snake scan of `x` × `y` quick tiles at `api`, and returns the seconds per tile. */
export async function secondsPerTile(request: APIRequestContext, api: string, x = 3, y = 2) {
  for (const [path, value] of [
    ['snake_workflow/x_count', x],
    ['snake_workflow/y_count', y],
    ['snake_workflow/capture_mode', 'quick'],
    ['smart_scan/workflow_name', 'snake_workflow'],
  ] as const) {
    await request.put(`${api}${path}`, {
      data: JSON.stringify(value),
      headers: { 'Content-Type': 'application/json' },
    })
  }
  const scan = await run(request, `${api}smart_scan/sample_scan`, {})
  if (scan.status !== 'completed') throw new Error(`the scan: ${JSON.stringify(scan.error)}`)
  const details = await (await request.get(`${api}smart_scan/latest_scan_live_details`)).json()
  return details.duration_seconds / details.image_count
}

/** How long, in ms, each tab takes to show once its rail link is clicked, measured in the page. */
export async function tabSwitches(page: Page, labels: string[]): Promise<number[]> {
  const times: number[] = []
  for (const label of labels) {
    times.push(
      await page.evaluate(async (label) => {
        const links = [...document.querySelectorAll<HTMLAnchorElement>('nav a')]
        const link = links.find((a) => a.textContent?.trim() === label)!
        const start = performance.now()
        link.click()
        await new Promise<void>((shown) => {
          const check = () =>
            document.querySelector('h1')?.textContent?.trim() === label
              ? shown()
              : requestAnimationFrame(check)
          check()
        })
        return performance.now() - start
      }, label),
    )
  }
  return times
}

/** A process's committed private memory and working set, in MB, and its handles, from Windows. */
export function processUse(pid: number) {
  const command = `Get-CimInstance Win32_Process -Filter 'ProcessId=${pid}' | Select-Object PrivatePageCount, WorkingSetSize, HandleCount | ConvertTo-Json`
  const out = execFileSync('powershell', ['-NoProfile', '-Command', command], { encoding: 'utf8' })
  const { PrivatePageCount, WorkingSetSize, HandleCount } = JSON.parse(out)
  const mb = (bytes: number) => bytes / 2 ** 20
  return { memoryMb: mb(PrivatePageCount), workingSetMb: mb(WorkingSetSize), handles: HandleCount }
}

/** Saves `data` as `name` in the test's results folder, which the nightly workflow keeps, and attaches it. */
export async function keep(info: TestInfo, name: string, data: unknown) {
  const file = info.outputPath(name)
  writeFileSync(file, JSON.stringify(data, null, 2))
  await info.attach(name, { path: file, contentType: 'application/json' })
}

/** The least-squares slope of `values` against their order: how much each adds, by the trend. */
export function slope(values: number[]) {
  const mean = (some: number[]) => some.reduce((a, b) => a + b, 0) / some.length
  const [mx, my] = [(values.length - 1) / 2, mean(values)]
  const spread = values.reduce((sum, _, i) => sum + (i - mx) ** 2, 0)
  return values.reduce((sum, y, i) => sum + (i - mx) * (y - my), 0) / spread
}

export const median = (values: number[]) => {
  const sorted = [...values].sort((a, b) => a - b)
  return sorted[Math.floor(sorted.length / 2)]!
}
