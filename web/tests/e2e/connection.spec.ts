import { spawn, type ChildProcess } from 'node:child_process'
import { fileURLToPath } from 'node:url'

import { expect, test } from '@playwright/test'

// The app on one origin, connected to a microscope server on another
// (through CORS), which stops and starts again.
const PORT = 5099
const ROOT = fileURLToPath(new URL('../../../', import.meta.url))
// The same binary as Playwright's server: MICROSCOPE_SERVER in CI, else the debug build.
const EXE = `${ROOT}${process.env.MICROSCOPE_SERVER ?? `target/debug/microscope-server${process.platform === 'win32' ? '.exe' : ''}`}`

async function startServer(): Promise<ChildProcess> {
  const server = spawn(EXE, ['-c', 'configs/simulation.json', '--port', String(PORT)], {
    cwd: ROOT,
    stdio: 'ignore',
  })
  await expect
    .poll(
      () =>
        fetch(`http://127.0.0.1:${PORT}/api/v1/health`).then(
          (r) => r.ok,
          () => false,
        ),
      {
        timeout: 15_000,
      },
    )
    .toBe(true)
  return server
}

test('connects to a microscope on another origin, and recovers after it restarts', async ({
  page,
}) => {
  test.setTimeout(60_000)
  let server = await startServer()
  try {
    await page.goto(`/?microscope=127.0.0.1:${PORT}#/view`)
    const indicator = page.locator('.connection-indicator')
    await expect(indicator).toHaveAttribute('data-state', 'connected')
    await expect(page).toHaveTitle(/ – Microscope$/)
    // Only the tabs whose Things the server has: it has no gallery yet.
    await expect(page.getByRole('link', { name: 'Gallery' })).toHaveCount(0)

    server.kill()
    await expect(page.getByRole('alert')).toContainText('Connection lost', { timeout: 15_000 })
    await expect(indicator).toHaveAttribute('data-state', /lost|reconnecting/)

    server = await startServer()
    await expect(indicator).toHaveAttribute('data-state', 'connected', { timeout: 20_000 })
    await expect(page.getByRole('alert')).toBeHidden()
  } finally {
    server.kill()
  }
})
