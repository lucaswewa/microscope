import { expect, test } from '@playwright/test'

import { framesPerSecond, keep, median, secondsPerTile, tabSwitches } from './measure'

// Performance budgets (P48), against the server under test: the preview's
// frame rate, a snake scan's time per tile, and how quickly the app changes
// tabs. The measurements are saved as budgets.json in the test's results,
// which the nightly workflow keeps.
test.use({ viewport: { width: 1280, height: 800 } })

const BUDGETS = {
  /** Whole frames a second, at least, from the main and low-resolution streams. */
  previewFps: 8,
  /**
   * Seconds, at most, for each tile of a quick snake scan: a smart stack at
   * each, its autofocus at the simulated stage's 500 z steps a second.
   */
  secondsPerTile: 12,
  /** Milliseconds, at most, for the median tab change, and the slowest. */
  tabSwitchMedianMs: 100,
  tabSwitchMaxMs: 500,
}

test('the preview, scans and the app stay within their budgets', async ({
  page,
  request,
}, info) => {
  test.setTimeout(300_000)
  const api = new URL('/api/v1/', info.project.use.baseURL!).href
  const preview = await framesPerSecond(`${api}camera/mjpeg_stream`)
  const lores = await framesPerSecond(`${api}camera/lores_mjpeg_stream`)
  const tile = await secondsPerTile(request, api)
  await request.put(`${api}smart_scan/workflow_name`, {
    data: JSON.stringify('histo_scan_workflow'),
    headers: { 'Content-Type': 'application/json' },
  })

  await page.goto('/#/view')
  await expect(page.locator('.connection-indicator')).toHaveAttribute('data-state', 'connected')
  const tabs = ['Control', 'Slide Scan', 'Sequence', 'Gallery', 'Settings', 'Logging', 'About']
  // Twice round: the second, with each page's code loaded, is what's measured.
  await tabSwitches(page, [...tabs, 'View'])
  const switches = await tabSwitches(page, [...tabs, 'View'])

  const measured = {
    previewFps: preview,
    loresFps: lores,
    secondsPerTile: tile,
    tabSwitchMedianMs: median(switches),
    tabSwitchMaxMs: Math.max(...switches),
  }
  await keep(info, 'budgets.json', { budgets: BUDGETS, measured })
  console.log(JSON.stringify(measured))
  expect(Math.min(preview, lores)).toBeGreaterThanOrEqual(BUDGETS.previewFps)
  expect(tile).toBeLessThanOrEqual(BUDGETS.secondsPerTile)
  expect(measured.tabSwitchMedianMs).toBeLessThanOrEqual(BUDGETS.tabSwitchMedianMs)
  expect(measured.tabSwitchMaxMs).toBeLessThanOrEqual(BUDGETS.tabSwitchMaxMs)
})
