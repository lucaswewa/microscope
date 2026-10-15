import { spawn, type ChildProcess } from 'node:child_process'
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

import { expect, type APIRequestContext } from '@playwright/test'

/**
 * Servers and launchers the end-to-end tests start themselves, beside the
 * one Playwright starts: each from a configuration of its own.
 */

export const ROOT = fileURLToPath(new URL('../../', import.meta.url))
const exe = process.platform === 'win32' ? '.exe' : ''
/** The server binary: MICROSCOPE_SERVER in CI, relative to the root, or else the debug build. */
export const SERVER = path.resolve(
  ROOT,
  process.env.MICROSCOPE_SERVER ?? `target/debug/microscope-server${exe}`,
)
/** The launcher, beside the server: MICROSCOPE_LAUNCHER in CI, or else the debug build. */
export const LAUNCHER = path.resolve(
  ROOT,
  process.env.MICROSCOPE_LAUNCHER ?? `target/debug/microscope-launcher${exe}`,
)

/** A configuration like the simulation's, keeping everything in a new folder. */
export function freshConfig(): { folder: string; config: string } {
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

/** Starts a server for `config` on `port`, once it answers. */
export async function startServer(config: string, port: number): Promise<ChildProcess> {
  const server = spawn(SERVER, ['-c', config, '--port', String(port)], {
    cwd: ROOT,
    stdio: 'ignore',
  })
  await expect
    .poll(
      () =>
        fetch(`http://127.0.0.1:${port}/api/v1/health`).then(
          (r) => r.ok,
          () => false,
        ),
      { timeout: 15_000 },
    )
    .toBe(true)
  return server
}

/** Runs an action at `url`, an action's address, and returns its finished invocation. */
export async function run(request: APIRequestContext, url: string, data: object = {}) {
  let invocation = await (await request.post(url, { data })).json()
  while (['pending', 'running'].includes(invocation.status)) {
    await new Promise((resolve) => setTimeout(resolve, 250))
    invocation = await (await request.get(invocation.href)).json()
  }
  return invocation
}
