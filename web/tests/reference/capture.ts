/// <reference lib="dom" />
// (The DOM types are for the page.evaluate callbacks, which run in the browser.)

/**
 * Captures reference screenshots for visual review (P05). Not run in CI.
 *
 *   node tests/reference/capture.ts --target ofm --url http://127.0.0.1:5095/
 *   node tests/reference/capture.ts --target microscope --url http://127.0.0.1:5173/
 *
 * Each state of the UI is captured in the light and dark themes (through the
 * system colour scheme, which both apps follow by default) at 1280×800 and
 * 1920×1080, into `docs/reference/<target>/<state>.<theme>.<width>x<height>.png`.
 * Both folders are git-ignored (ADR-0002). `docs/reference/compare.html`
 * shows the two targets side by side.
 *
 * With `--target ofm`, the script also prepares OpenFlexure's simulator through
 * its API (calibrations, captures, a short scan) and records layout
 * measurements in `docs/reference/ofm/measurements.json`. See
 * `docs/reference/README.md`.
 */
import { mkdir, writeFile } from 'node:fs/promises'
import { fileURLToPath } from 'node:url'
import { parseArgs } from 'node:util'

import { chromium, type Browser, type Page } from '@playwright/test'

type Target = 'ofm' | 'microscope'
type Theme = 'light' | 'dark'
interface Viewport {
  width: number
  height: number
}

const THEMES: Theme[] = ['light', 'dark']
const VIEWPORTS: Viewport[] = [
  { width: 1280, height: 800 },
  { width: 1920, height: 1080 },
]
/** Time for streams, images and transitions to settle before a screenshot. */
const SETTLE_MS = 2000

/** A state of the UI: how to reach it in OpenFlexure, and its route in this app. */
interface Shot {
  name: string
  ofm: (page: Page) => Promise<void>
  route: string
}

const ofmTab = (id: string) => async (page: Page) => {
  await page.click(`#${id}-tab-icon`)
}
const ofmSettings = (section: string) => async (page: Page) => {
  await page.click('#settings-tab-icon')
  await page.locator('.settings-nav').getByText(section, { exact: true }).click()
}

const SHOTS: Shot[] = [
  { name: 'view', ofm: ofmTab('view'), route: '/#/view' },
  { name: 'control', ofm: ofmTab('control'), route: '/#/control' },
  { name: 'slide-scan', ofm: ofmTab('slide-scan'), route: '/#/slide-scan' },
  { name: 'sequence', ofm: ofmTab('timelapse'), route: '/#/sequence' },
  { name: 'settings-display', ofm: ofmSettings('Display'), route: '/#/settings/display' },
  {
    name: 'settings-stage-control',
    ofm: ofmSettings('Stage Control Preferences'),
    route: '/#/settings/stage-control',
  },
  { name: 'settings-camera', ofm: ofmSettings('Camera'), route: '/#/settings/camera' },
  { name: 'settings-stage', ofm: ofmSettings('Stage'), route: '/#/settings/stage' },
  {
    name: 'settings-mapping',
    ofm: ofmSettings('Camera to Stage Mapping'),
    route: '/#/settings/mapping',
  },
  { name: 'logging', ofm: ofmTab('logging'), route: '/#/logging' },
  { name: 'about', ofm: ofmTab('about'), route: '/#/about' },
  { name: 'power', ofm: ofmTab('power'), route: '/#/power' },
]
const GALLERY: Shot = { name: 'gallery', ofm: ofmTab('gallery'), route: '/#/gallery' }
const SCAN_RUNNING: Shot = {
  name: 'slide-scan-running',
  ofm: ofmTab('slide-scan'),
  route: '/#/slide-scan',
}
/** The wizard's welcome page, which opens by itself while calibrations are missing. */
const WIZARD_WELCOME: Shot = {
  name: 'calibration-wizard',
  ofm: async () => {},
  route: '/#/calibration-wizard',
}
/** The wizard as Settings launches it, with every task and no welcome page. */
const WIZARD_FROM_SETTINGS: Shot = {
  name: 'calibration-wizard-from-settings',
  ofm: async (page) => {
    await ofmSettings('Display')(page)
    await page.getByRole('button', { name: 'Launch Calibration Wizard' }).click()
  },
  route: '/#/settings/calibration-wizard',
}
const ALL_SHOTS = [...SHOTS, GALLERY, SCAN_RUNNING, WIZARD_WELCOME, WIZARD_FROM_SETTINGS]

const { values: args } = parseArgs({
  options: {
    target: { type: 'string', default: 'ofm' },
    url: { type: 'string' },
  },
})
const target = args.target as Target
if (target !== 'ofm' && target !== 'microscope') {
  throw new Error(`--target must be ofm or microscope, not ${args.target}`)
}
const appUrl = new URL(args.url ?? 'http://127.0.0.1:5095/')
const referenceDir = new URL('../../../docs/reference/', import.meta.url)
const outDir = new URL(`${target}/`, referenceDir)
await mkdir(outDir, { recursive: true })

const fileName = (shot: Shot, theme: Theme, viewport: Viewport) =>
  `${shot.name}.${theme}.${viewport.width}x${viewport.height}.png`

/** Saves a screenshot of `page` as `shot` in this theme and viewport. */
async function save(page: Page, shot: Shot, theme: Theme, viewport: Viewport) {
  const name = fileName(shot, theme, viewport)
  await page.screenshot({ path: fileURLToPath(new URL(name, outDir)) })
  console.log(`captured ${target}/${name}`)
}

/** Opens the app in a new page with the given theme and viewport. */
async function open(browser: Browser, theme: Theme, viewport: Viewport): Promise<Page> {
  const context = await browser.newContext({ viewport, colorScheme: theme })
  const page = await context.newPage()
  await page.goto(appUrl.href)
  await page.waitForTimeout(SETTLE_MS)
  return page
}

/** Brings `page` to `shot`'s state and saves a screenshot. */
async function capture(page: Page, shot: Shot, theme: Theme, viewport: Viewport) {
  if (target === 'ofm') {
    await shot.ofm(page)
  } else {
    await page.goto(new URL(shot.route, appUrl).href)
  }
  // Move the pointer off whatever was clicked, so no tooltip or hover shows.
  await page.mouse.move(viewport.width - 1, viewport.height - 1)
  await page.waitForTimeout(SETTLE_MS)
  await save(page, shot, theme, viewport)
}

/** Calls each theme and viewport in turn with a fresh page. */
async function eachVariant(
  browser: Browser,
  run: (page: Page, theme: Theme, viewport: Viewport) => Promise<void>,
) {
  for (const theme of THEMES) {
    for (const viewport of VIEWPORTS) {
      const page = await open(browser, theme, viewport)
      await run(page, theme, viewport)
      await page.context().close()
    }
  }
}

// ---- OpenFlexure's API, to prepare the simulator ----------------------------

const api = (path: string) => new URL(`/api/v3/${path}`, appUrl).href

async function writeProperty(path: string, value: unknown) {
  const response = await fetch(api(path), {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(value),
  })
  if (!response.ok) throw new Error(`PUT ${path}: ${response.status}`)
}

/** Starts an action and returns the URL of its invocation. */
async function startAction(path: string, input: object = {}): Promise<string> {
  const response = await fetch(api(path), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(input),
  })
  if (!response.ok) throw new Error(`POST ${path}: ${response.status} ${await response.text()}`)
  const invocation = (await response.json()) as { href: string }
  return new URL(invocation.href, appUrl).href
}

/** Waits for an invocation to finish, and fails unless it completed. */
async function finish(href: string, timeoutMs = 300_000) {
  const deadline = Date.now() + timeoutMs
  while (Date.now() < deadline) {
    const invocation = (await (await fetch(href)).json()) as { status: string }
    if (invocation.status === 'completed') return
    if (['error', 'cancelled'].includes(invocation.status)) {
      throw new Error(`${href} ended ${invocation.status}`)
    }
    await new Promise((resolve) => setTimeout(resolve, 500))
  }
  throw new Error(`${href} didn't finish in ${timeoutMs / 1000} s`)
}

async function prepareOfm() {
  console.log('calibrating the camera and the camera-stage mapping...')
  await finish(await startAction('camera/full_auto_calibrate'))
  await finish(await startAction('camera_stage_mapping/calibrate_xy'))
  console.log('capturing images for the gallery...')
  for (let i = 0; i < 3; i++) {
    // OpenFlexure names captures to the second, so two in one second collide.
    await new Promise((resolve) => setTimeout(resolve, 1100))
    await finish(await startAction('camera/capture'))
  }
  // A short scan: three by two fields.
  await writeProperty('snake_workflow/x_count', 3)
  await writeProperty('snake_workflow/y_count', 2)
}

// ---- Measurements -------------------------------------------------------------

/** Layout and colours of OpenFlexure's main elements, measured in the page. */
async function measure(page: Page) {
  return page.evaluate(() => {
    const props = [
      'width',
      'height',
      'padding',
      'margin',
      'font-family',
      'font-size',
      'font-weight',
      'line-height',
      'color',
      'background-color',
      'border-color',
      'border-width',
      'border-radius',
    ]
    // OpenFlexure keeps every tab in the page and hides the inactive ones, so
    // the first visible match is the one on screen.
    const element = (selector: string) => {
      const matches = [...document.querySelectorAll(selector)]
      const el = matches.find((m) => m.getBoundingClientRect().width > 0) ?? matches[0]
      if (!el) return null
      const style = getComputedStyle(el)
      const box = el.getBoundingClientRect()
      return {
        box: { width: Math.round(box.width), height: Math.round(box.height) },
        ...Object.fromEntries(props.map((p) => [p, style.getPropertyValue(p)])),
      }
    }
    const selectors = [
      'body',
      // Vue's root element, inside the mount point, carries the theme.
      '#app > #app',
      '#container-left',
      '#switcher-left-container',
      '#switcher-left',
      '#view-tab-icon',
      '#control-tab-icon',
      '#control-tab-icon .material-symbols-outlined',
      '.control-component',
      '.view-component',
      '.uk-button-primary',
      '.uk-button-default',
      '.uk-select',
      '.uk-input',
      '.uk-accordion-title',
      '.menu-subheading',
      '.dpad-btn',
      'h1',
      'h2',
      'h3',
    ]
    return Object.fromEntries(selectors.map((s) => [s, element(s)]))
  })
}

// ---- The run ----------------------------------------------------------------

const browser = await chromium.launch()
const measurements: Record<string, unknown> = {}

if (target === 'ofm') {
  // The wizard's welcome page only opens while calibrations are missing, so
  // it is captured first (and kept from an earlier run otherwise).
  await eachVariant(browser, async (page, theme, viewport) => {
    const opened = await page
      .locator('.uk-modal.uk-open')
      .waitFor({ timeout: 5000 })
      .then(
        () => true,
        () => false,
      )
    if (opened) await save(page, WIZARD_WELCOME, theme, viewport)
  })
  await prepareOfm()
  await eachVariant(browser, async (page, theme, viewport) => {
    for (const shot of SHOTS) await capture(page, shot, theme, viewport)
    await capture(page, WIZARD_FROM_SETTINGS, theme, viewport)
    // Close the wizard, which asks before closing.
    await page.reload()
    await page.waitForTimeout(SETTLE_MS)
    if (viewport.width === 1280) {
      for (const tab of ['control', 'slide-scan']) {
        await page.click(`#${tab}-tab-icon`)
        await page.waitForTimeout(SETTLE_MS)
        measurements[`${tab}.${theme}`] = await measure(page)
      }
      await ofmSettings('Camera')(page)
      await page.waitForTimeout(SETTLE_MS)
      measurements[`settings-camera.${theme}`] = await measure(page)
    }
  })
  console.log('starting a short snake scan...')
  await writeProperty('smart_scan/workflow_name', 'snake_workflow')
  const scan = await startAction('smart_scan/sample_scan', { scan_name: 'reference' })
  await eachVariant(browser, (page, theme, viewport) =>
    capture(page, SCAN_RUNNING, theme, viewport),
  )
  console.log('waiting for the scan to finish...')
  await finish(scan, 600_000).catch((error) => console.warn(`the scan: ${error}`))
  await writeProperty('smart_scan/workflow_name', 'histo_scan_workflow')
  await eachVariant(browser, (page, theme, viewport) => capture(page, GALLERY, theme, viewport))
  await writeFile(new URL('measurements.json', outDir), JSON.stringify(measurements, null, 2))
  console.log(`measured ofm/measurements.json`)
} else {
  await eachVariant(browser, async (page, theme, viewport) => {
    for (const shot of ALL_SHOTS) await capture(page, shot, theme, viewport)
  })
}
await browser.close()

// ---- The comparison page ------------------------------------------------------

const rows = ALL_SHOTS.flatMap((shot) =>
  THEMES.flatMap((theme) =>
    VIEWPORTS.map((viewport) => {
      const name = fileName(shot, theme, viewport)
      return `<h2>${name}</h2><div class="pair"><figure><figcaption>OpenFlexure</figcaption><img src="ofm/${name}" alt="not captured"></figure><figure><figcaption>This app</figcaption><img src="microscope/${name}" alt="not captured"></figure></div>`
    }),
  ),
)
await writeFile(
  new URL('compare.html', referenceDir),
  `<!doctype html><meta charset="utf-8"><title>Reference comparison</title>
<style>body{font-family:system-ui,sans-serif;margin:1rem}h2{font-size:1rem;margin:1.5rem 0 .5rem}.pair{display:grid;grid-template-columns:1fr 1fr;gap:1rem}figure{margin:0}img{width:100%;border:1px solid #ccc}</style>
<h1>Reference comparison</h1>${rows.join('\n')}\n`,
)
console.log('wrote docs/reference/compare.html')
