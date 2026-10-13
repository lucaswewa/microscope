import { spawn, type ChildProcess } from 'node:child_process'
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

import { expect, test } from '@playwright/test'

// The app on one origin, connected to a fresh microscope server on another
// (through CORS), with its own settings folder. The tests share its port, so
// they run in turn.
test.describe.configure({ mode: 'default' })

const PORT = 5099
const ROOT = fileURLToPath(new URL('../../../', import.meta.url))
// The same binary as Playwright's server: MICROSCOPE_SERVER in CI, else the debug build.
const EXE = path.resolve(
  ROOT,
  process.env.MICROSCOPE_SERVER ??
    `target/debug/microscope-server${process.platform === 'win32' ? '.exe' : ''}`,
)

/** A configuration like the simulation's, keeping everything in a new folder. */
function freshConfig(): { folder: string; config: string } {
  const folder = mkdtempSync(path.join(tmpdir(), 'microscope-e2e-'))
  const config = JSON.parse(readFileSync(path.join(ROOT, 'configs/simulation.json'), 'utf8'))
  config.settings_folder = path.join(folder, 'settings')
  config.application_config = {
    data_folder: path.join(folder, 'data'),
    log_folder: path.join(folder, 'logs'),
  }
  writeFileSync(path.join(folder, 'config.json'), JSON.stringify(config))
  return { folder, config: path.join(folder, 'config.json') }
}

async function startServer(config: string): Promise<ChildProcess> {
  const server = spawn(EXE, ['-c', config, '--port', String(PORT)], { cwd: ROOT, stdio: 'ignore' })
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

let fresh: { folder: string; config: string }
test.beforeEach(() => (fresh = freshConfig()))
test.afterEach(() => rmSync(fresh.folder, { recursive: true, force: true, maxRetries: 5 }))

test('connects to a microscope on another origin, and recovers after it restarts', async ({
  page,
}) => {
  test.setTimeout(60_000)
  let server = await startServer(fresh.config)
  try {
    await page.goto(`/?microscope=127.0.0.1:${PORT}#/view`)
    const indicator = page.locator('.connection-indicator')
    await expect(indicator).toHaveAttribute('data-state', 'connected')
    await expect(page).toHaveTitle(/ – Microscope$/)
    // It needs calibrating, so the calibration wizard opens; Escape closes it, after asking.
    await expect(page.getByRole('dialog')).toContainText('Welcome to your microscope')
    await page.keyboard.press('Escape')
    await page.getByRole('button', { name: 'Close the wizard' }).click()
    await expect(page.getByRole('dialog')).toBeHidden()
    // Only the tabs whose Things the server has: it has a gallery, but no
    // sequence yet.
    await expect(page.getByRole('link', { name: 'Gallery' })).toHaveCount(1)
    await expect(page.getByRole('link', { name: 'Sequence' })).toHaveCount(0)

    server.kill()
    await expect(page.getByRole('alert')).toContainText('Connection lost', { timeout: 15_000 })
    await expect(indicator).toHaveAttribute('data-state', /lost|reconnecting/)

    server = await startServer(fresh.config)
    await expect(indicator).toHaveAttribute('data-state', 'connected', { timeout: 20_000 })
    await expect(page.getByRole('alert')).toBeHidden()
    // Reconnecting doesn't open the wizard again.
    await expect(page.getByRole('dialog')).toBeHidden()
  } finally {
    server.kill()
  }
})

test('the calibration wizard calibrates a fresh microscope', async ({ page }) => {
  test.setTimeout(180_000)
  const server = await startServer(fresh.config)
  try {
    await page.goto(`/?microscope=127.0.0.1:${PORT}#/view`)
    // The wizard, not the progress dialogs its calibrations open over it.
    const wizard = page.getByRole('dialog', { name: /^(Welcome|Calibration)/ })
    // Next, once it doesn't say Skip.
    const next = () => wizard.getByRole('button', { name: /^(Next|Finish)$/ }).click()
    await expect(wizard).toHaveAccessibleName('Welcome to your microscope')
    await next()
    await wizard.getByRole('button', { name: 'Confirm the z direction' }).click()
    await next()
    for (const step of ['illumination', 'centring', 'an empty view']) {
      await expect(wizard, step).toHaveAccessibleName('Calibration: Camera')
      await next()
    }
    await wizard.getByRole('button', { name: 'Full auto-calibrate' }).click()
    await next()
    await expect(wizard).toHaveAccessibleName('Calibration: Camera–stage mapping')
    await next()
    await wizard.getByRole('button', { name: 'Calibrate' }).click()
    await next()
    await expect(wizard).toContainText("That's the calibration done.")
    await next()
    await expect(wizard).toBeHidden()

    for (const thing of ['stage', 'camera', 'camera_stage_mapping']) {
      const url = `http://127.0.0.1:${PORT}/api/v1/${thing}/calibration_required`
      expect(await (await page.request.get(url)).json(), thing).toBe(false)
    }
  } finally {
    server.kill()
  }
})
