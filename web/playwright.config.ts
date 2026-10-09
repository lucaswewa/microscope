import path from 'node:path'
import { fileURLToPath } from 'node:url'

import { defineConfig, devices } from '@playwright/test'

const port = 5098
const root = fileURLToPath(new URL('..', import.meta.url))

// The server under test serves the app at / (ADR-0016). By default it's a
// debug build of this checkout, serving the web app's fresh build from
// web/dist. CI sets MICROSCOPE_SERVER, relative to the repository's root, to
// a release build with the app embedded.
const serverCommand = process.env.MICROSCOPE_SERVER
  ? `"${path.resolve(root, process.env.MICROSCOPE_SERVER)}" -c configs/simulation.json --port ${port}`
  : `npm --prefix web run build && cargo run -q -p microscope-server -- -c configs/simulation.json --port ${port} --webapp-dir web/dist`

// End-to-end and visual tests, against the app as the server serves it.
// (tests/unit is Vitest's.)
export default defineConfig({
  testDir: './tests',
  testMatch: /(e2e|visual)\/.*\.spec\.ts$/,
  expect: {
    // Allow for small differences in font rendering between machines.
    toHaveScreenshot: { maxDiffPixelRatio: 0.01 },
  },
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  reporter: 'list',
  use: {
    baseURL: `http://127.0.0.1:${port}`,
    trace: 'on-first-retry',
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
  webServer: {
    command: serverCommand,
    cwd: root,
    url: `http://127.0.0.1:${port}/api/v1/health`,
    reuseExistingServer: !process.env.CI,
    // The first run builds the server.
    timeout: 600_000,
  },
})
