import { defineConfig, devices } from '@playwright/test'

const port = 4173

// End-to-end and visual tests run against the production build, served by
// `vite preview`. (tests/unit is Vitest's.)
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
  webServer: {
    command: `npm run build && npm run preview -- --port ${port} --strictPort`,
    port,
    reuseExistingServer: !process.env.CI,
  },
})
