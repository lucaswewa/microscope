import { mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

import { defineConfig, devices } from '@playwright/test'

const port = 5098
const root = fileURLToPath(new URL('..', import.meta.url))

/**
 * The simulation's configuration, keeping the server's settings, data and
 * logs in a folder of its own, emptied for each run: the tests start from a
 * fresh microscope, as in CI, and leave the repository's `.microscope`
 * alone. Its path, quoted.
 */
function freshConfig() {
  const folder = path.join(tmpdir(), 'microscope-e2e-server')
  const file = path.join(folder, 'config.json')
  // The workers load this file too; only the runner, which starts the
  // server, writes it.
  if (process.env.TEST_WORKER_INDEX === undefined) {
    rmSync(folder, { recursive: true, force: true })
    mkdirSync(folder, { recursive: true })
    const config = JSON.parse(readFileSync(path.join(root, 'configs/simulation.json'), 'utf8'))
    config.settings_folder = path.join(folder, 'settings')
    config.application_config = {
      data_folder: path.join(folder, 'data'),
      log_folder: path.join(folder, 'logs'),
    }
    writeFileSync(file, JSON.stringify(config))
  }
  return `"${file}"`
}

// The server under test serves the app at / (ADR-0016). By default it's a
// debug build of this checkout, serving the web app's fresh build from
// web/dist. CI sets MICROSCOPE_SERVER, relative to the repository's root, to
// a release build with the app embedded.
const config = freshConfig()
const serverCommand = process.env.MICROSCOPE_SERVER
  ? `"${path.resolve(root, process.env.MICROSCOPE_SERVER)}" -c ${config} --port ${port}`
  : `npm --prefix web run build && cargo run -q -p microscope-server -- -c ${config} --port ${port} --webapp-dir web/dist`

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
  projects: [
    // Calibrates the server first, so the calibration wizard stays shut.
    { name: 'setup', testMatch: /e2e\/calibrate\.setup\.ts$/ },
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
      dependencies: ['setup'],
      testIgnore: /e2e\/(scan|sequence)\.spec\.ts$/,
    },
    // A scan holds the microscope for a while, so it runs once the others have finished.
    {
      name: 'scan',
      use: { ...devices['Desktop Chrome'] },
      dependencies: ['chromium'],
      testMatch: /e2e\/scan\.spec\.ts$/,
    },
    // So does a sequence, after the scan.
    {
      name: 'sequence',
      use: { ...devices['Desktop Chrome'] },
      dependencies: ['scan'],
      testMatch: /e2e\/sequence\.spec\.ts$/,
    },
  ],
  webServer: {
    command: serverCommand,
    cwd: root,
    url: `http://127.0.0.1:${port}/api/v1/health`,
    reuseExistingServer: !process.env.CI,
    // The first run builds the server.
    timeout: 600_000,
  },
})
