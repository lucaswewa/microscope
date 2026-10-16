import { execFileSync, spawn } from 'node:child_process'
import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { createInterface } from 'node:readline'

import { expect, test } from '@playwright/test'

import { LAUNCHER, ROOT } from '../servers'

// Restart through the launcher (P45, P46, P48): the app, served by the
// server the launcher started, says it's managed, and Power's Restart brings
// the server back as a new process, on the same address, which the app
// reconnects to.
test.use({ viewport: { width: 1280, height: 800 } })

test('Restart from the Power tab goes through the launcher, and the app reconnects', async ({
  page,
}) => {
  test.setTimeout(240_000)
  // Locally, the debug build, beside the server Playwright's runs build; CI
  // builds release ones. Only the launcher is built here: other tests run
  // the server's binary meanwhile.
  if (!process.env.MICROSCOPE_LAUNCHER) {
    execFileSync('cargo', ['build', '-q', '--locked', '-p', 'microscope-launcher'], {
      cwd: ROOT,
      stdio: 'inherit',
    })
  }
  const root = mkdtempSync(path.join(tmpdir(), 'microscope-launcher-'))
  const launcher = spawn(LAUNCHER, ['--root', root, '--port', '0', '--no-browser'], {
    cwd: ROOT,
    stdio: ['ignore', 'pipe', 'ignore'],
  })
  const ready: { url: string; pid: number }[] = []
  createInterface({ input: launcher.stdout! }).on('line', (line) => {
    if (line.startsWith('MICROSCOPE_READY ')) ready.push(JSON.parse(line.slice(17)))
  })
  try {
    await expect.poll(() => ready.length, { timeout: 60_000 }).toBe(1)
    const [first] = ready
    await page.goto(`${first!.url}/#/about`)
    // A fresh microscope: its calibration wizard is closed, after asking.
    await expect(page.getByRole('dialog')).toContainText('Welcome to your microscope')
    await page.keyboard.press('Escape')
    await page.getByRole('button', { name: 'Close the wizard' }).click()
    const managed = page.locator('dt', { hasText: 'Managed by the launcher' }).locator('+ dd')
    await expect(managed).toHaveText('Yes: it can restart')

    await page.getByRole('link', { name: 'Power' }).click()
    await page.getByRole('button', { name: 'Restart' }).click()
    await page.getByRole('alertdialog').getByRole('button', { name: 'Restart' }).click()
    await expect.poll(() => ready.length, { timeout: 60_000 }).toBe(2)
    expect(ready[1]!.pid).not.toBe(first!.pid)
    expect(ready[1]!.url).toBe(first!.url)
    await expect(page.locator('.connection-indicator')).toHaveAttribute('data-state', 'connected', {
      timeout: 30_000,
    })
  } finally {
    // Its Job Object ends the server with it (ADR-0033).
    if (launcher.exitCode === null) {
      const exited = new Promise((resolve) => launcher.once('exit', resolve))
      launcher.kill()
      await exited
    }
    rmSync(root, { recursive: true, force: true, maxRetries: 10 })
  }
})
