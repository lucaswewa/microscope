import { rmSync } from 'node:fs'

import { expect, test } from '@playwright/test'

import { freshConfig, run, startServer } from '../servers'
import { keep, processUse, secondsPerTile, slope } from './measure'

// The soak test (P48): a fresh server streams its live image to the app and
// scans, stitching each scan, over and over, for MICROSCOPE_SOAK_MINUTES (30
// by default). Its memory and handles, sampled after each scan, mustn't
// grow once it has warmed up. The samples are saved as soak.json in the
// test's results.
const MINUTES = Number(process.env.MICROSCOPE_SOAK_MINUTES ?? 30)
const PORT = 5099

test.use({ viewport: { width: 1280, height: 800 } })

test('streaming and scanning for half an hour, the server’s memory and handles stay steady', async ({
  page,
  request,
}, info) => {
  test.setTimeout((MINUTES + 15) * 60_000)
  const fresh = freshConfig()
  const server = await startServer(fresh.config, PORT)
  const api = `http://127.0.0.1:${PORT}/api/v1/`
  try {
    const inverted = await (await request.get(`${api}stage/axis_inverted`)).json()
    for (const [action, input] of [
      ['stage/calibrate_z_direction', { z_inverted: inverted.z }],
      ['camera/full_auto_calibrate', {}],
      ['camera_stage_mapping/calibrate_xy', {}],
    ] as const) {
      expect((await run(request, `${api}${action}`, input)).status, action).toBe('completed')
    }
    // The live image streams throughout, from the app on its usual origin.
    await page.goto(`/?microscope=127.0.0.1:${PORT}#/view`)
    await expect(page.locator('.live-image')).toHaveAttribute('data-status', 'live')

    const samples: ({ minute: number; secondsPerTile: number } & ReturnType<typeof processUse>)[] =
      []
    const start = Date.now()
    while (Date.now() - start < MINUTES * 60_000) {
      const tile = await secondsPerTile(request, api, 2, 2)
      const minute = (Date.now() - start) / 60_000
      samples.push({ minute, secondsPerTile: tile, ...processUse(server.pid!) })
    }
    await keep(info, 'soak.json', samples)
    await expect(page.locator('.live-image')).toHaveAttribute('data-status', 'live')

    // After a fifth to warm up, the growth over the rest, by its trend: a
    // leak of a megabyte, or a handle every five scans, fails half an hour.
    const steady = samples.slice(Math.ceil(samples.length / 5))
    const growth = (key: 'memoryMb' | 'handles') =>
      slope(steady.map((sample) => sample[key])) * steady.length
    console.log(JSON.stringify({ scans: samples.length, memoryMb: growth('memoryMb') }))
    expect(samples.length).toBeGreaterThanOrEqual(4)
    expect(growth('memoryMb')).toBeLessThanOrEqual(16)
    expect(growth('handles')).toBeLessThanOrEqual(8)
  } finally {
    server.kill()
    rmSync(fresh.folder, { recursive: true, force: true, maxRetries: 10 })
  }
})
