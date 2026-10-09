import { defineConfig, devices } from '@playwright/test'

const port = 4173
/** The microscope server the app connects to, through the preview's proxy. */
const serverPort = 5098

// End-to-end and visual tests run against the production build, served by
// `vite preview`, connected to a microscope server built from this checkout.
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
    baseURL: `http://localhost:${port}`,
    trace: 'on-first-retry',
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
  webServer: [
    {
      command: `cargo run -q -p microscope-server -- -c configs/simulation.json --port ${serverPort}`,
      cwd: '..',
      url: `http://127.0.0.1:${serverPort}/api/v1/health`,
      reuseExistingServer: !process.env.CI,
      // The first run builds the server.
      timeout: 600_000,
    },
    {
      command: `npm run build && npm run preview -- --port ${port} --strictPort`,
      port,
      reuseExistingServer: !process.env.CI,
      env: { MICROSCOPE_API_TARGET: `http://127.0.0.1:${serverPort}` },
    },
  ],
})
