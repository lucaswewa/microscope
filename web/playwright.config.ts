import { defineConfig, devices } from '@playwright/test'

const port = 4173

// End-to-end tests run against the production build, served by `vite preview`.
export default defineConfig({
  testDir: './tests/e2e',
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
