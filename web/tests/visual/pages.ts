import { fileURLToPath } from 'node:url'

import { expect, type Locator, type Page } from '@playwright/test'

import { LOG } from '../unit/logging/fixture'

/**
 * Every tab and Settings section, and a few key states, as the visual and
 * accessibility tests open them (ADR-0034), with what changes from run to
 * run fixed: the live stream off, and the stage and its measured range, the
 * computer, the gallery, the log, and the scan's and sequence's settings
 * answered by the test, whatever a server has saved. Use them with
 * `STEADY`: one time zone and locale, and no transitions, which the app
 * drops for reduced motion.
 */
export const STEADY = { timezoneId: 'UTC', locale: 'en-US', reducedMotion: 'reduce' } as const

const IMAGE = fileURLToPath(new URL('../fixtures/dzi/sample_files/9/0_0.jpeg', import.meta.url))

const card = (name: string, type: string, info: [string, string][]) => ({
  name,
  path: `${type.toLowerCase()}s/${name}`,
  card_type: type,
  created: '2026-10-14T09:30:00.000Z',
  thumbnail: `${name}.jpeg`,
  viewer: { type: 'image', path: `${name}.jpeg` },
  info: info.map(([label, value]) => ({ label, value })),
  download_buttons: [{ label: 'Image', source: 'data', path: `${name}.jpeg`, filename: name }],
  card_actions: [],
  card_messages: [],
  delete_endpoint: `gallery/${type.toLowerCase()}s/${name}`,
})
const GALLERY = [
  card('slide-1', 'Scan', [
    ['Images', '24'],
    ['Workflow', 'Histo scan'],
  ]),
  card('lapse', 'Sequence', [
    ['Images', '20'],
    ['Interval', '30 s'],
  ]),
  card('capture-1', 'Capture', [['Size', '1640 × 1232 px']]),
]

/** Answers the requests whose answers change from run to run. */
export async function fix(page: Page) {
  await page.addInitScript(() => localStorage.setItem('microscope.stream-disabled', 'true'))
  const answer = (path: string, json: unknown) =>
    page.route(`**/api/v1/${path}`, (route) => route.fulfill({ json }))
  await answer('stage/position', { x: 0, y: 0, z: 0 })
  await answer('stage/moving', false)
  await answer('stage/axis_inverted', { x: false, y: false, z: false })
  await answer('stage_measure/calibrated_range', null)
  await answer('system/hostname', 'lab-pc')
  await answer('system/version_data', { version: '0.2.0', version_source: 'v0.2.0' })
  await answer('system/os_version', 'Windows 11')
  await answer('gallery/list_data', GALLERY)
  // As they start, whatever the scan and sequence tests leave.
  await answer('smart_scan/workflow_name', 'histo_scan_workflow')
  await answer('sequence/duration', 600)
  await answer('sequence/dt', 30)
  await answer('sequence/autofocus_enabled', false)
  await answer('sequence/autofocus_dz', 2000)
  await answer('log/', LOG)
  await page.route('**/api/v1/data/**', (route) => route.fulfill({ path: IMAGE }))
}

export interface Shot {
  name: string
  route: string
  /** More answers, for this shot alone. */
  answers?: (page: Page) => Promise<void>
  /** What shows once the page has loaded. */
  ready: (page: Page) => Locator
  /** A state reached from the page, such as an open dialog. */
  then?: (page: Page) => Promise<Locator>
  /** Also at 1920 × 1080. */
  large?: boolean
}
const text = (words: string) => (page: Page) => page.getByText(words, { exact: true }).first()

/**
 * A scan running, as the Slide Scan tab finds it, with its preview: the
 * fixture's picture as four images in a snake, overlapping, and where each
 * went (P53).
 */
async function scanning(page: Page) {
  const record = (url: string) => ({
    id: 'visual-scan',
    status: 'running',
    action: '/api/v1/smart_scan/sample_scan',
    href: new URL('/api/v1/action_invocations/visual-scan', url).href,
    log: [{ message: 'Scanning with snake_workflow from x 0, y 0, z 0' }],
  })
  await page.route('**/api/v1/smart_scan/sample_scan', (route) =>
    route.fulfill({ json: [record(route.request().url())] }),
  )
  await page.route('**/api/v1/action_invocations/visual-scan', (route) =>
    route.fulfill({ json: record(route.request().url()) }),
  )
  await page.route('**/api/v1/smart_scan/latest_scan_live_details', (route) =>
    route.fulfill({
      json: {
        name: 'slide-2',
        scan_phase: 'scanning',
        image_count: 4,
        duration_seconds: 75,
        workflow: 'snake_workflow',
        settings: { x_count: 2, y_count: 2 },
        result: null,
        preview_stitch_time: '2026-10-14T09:30:00.000Z',
      },
    }),
  )
  await page.route('**/api/v1/smart_scan/latest_preview_stitch.jpg*', (route) =>
    route.fulfill({ path: IMAGE }),
  )
  const tile = (number: number, x: number, y: number) => ({
    image: `img_${x}_${y}_0.jpeg`,
    number,
    x,
    y,
    width: 160,
    height: 125,
  })
  await page.route('**/api/v1/smart_scan/latest_preview_tiles.json*', (route) =>
    route.fulfill({
      json: {
        size: [256, 200],
        tiles: [tile(1, 0, 0), tile(2, 96, 0), tile(3, 96, 75), tile(4, 0, 75)],
      },
    }),
  )
}
export const SHOTS: Shot[] = [
  { name: 'view', route: 'view', ready: text('Turn it on') },
  { name: 'control', route: 'control', ready: text('On capture'), large: true },
  { name: 'slide-scan', route: 'slide-scan', ready: text('Start Smart Scan'), large: true },
  {
    name: 'slide-scan-outlines',
    route: 'slide-scan',
    answers: scanning,
    ready: (page) => page.locator('.overlay-layers [data-layer="image-outlines"] > g').nth(3),
  },
  { name: 'sequence', route: 'sequence', ready: text('Autofocus range (steps)') },
  { name: 'gallery', route: 'gallery', ready: text('capture-1'), large: true },
  {
    name: 'gallery-viewer',
    route: 'gallery',
    ready: text('capture-1'),
    then: async (page) => {
      await page.getByRole('button', { name: 'View capture-1' }).click()
      return page.locator('.deep-zoom[data-status="shown"]')
    },
  },
  { name: 'settings-display', route: 'settings/display', ready: text('Turn the live image off') },
  { name: 'settings-stage-control', route: 'settings/stage-control', ready: text('x step') },
  { name: 'settings-connection', route: 'settings/connection', ready: text('Another microscope') },
  {
    name: 'settings-camera',
    route: 'settings/camera',
    ready: text('Full auto-calibrate'),
    large: true,
  },
  { name: 'settings-stage', route: 'settings/stage', ready: text('Invert z') },
  {
    name: 'settings-camera-stage-mapping',
    route: 'settings/camera-stage-mapping',
    ready: (page) => page.getByRole('table', { name: 'The image-to-stage matrix' }),
  },
  {
    name: 'calibration-wizard',
    route: 'settings/display',
    ready: text('Turn the live image off'),
    then: async (page) => {
      await page.getByRole('button', { name: 'Calibration wizard' }).click()
      return page.getByRole('dialog')
    },
  },
  { name: 'logging', route: 'logging', ready: text('sequence.run_sequence') },
  { name: 'about', route: 'about', ready: text('Windows 11') },
  { name: 'power', route: 'power', ready: text('Shut down') },
  {
    name: 'power-shut-down',
    route: 'power',
    ready: text('Shut down'),
    then: async (page) => {
      await page.getByRole('button', { name: 'Shut down' }).click()
      return page.getByRole('alertdialog')
    },
  },
]

/** Opens `shot`'s page, and its state, once it has loaded. */
export async function open(page: Page, shot: Shot) {
  await fix(page)
  await shot.answers?.(page)
  await page.goto(`/#/${shot.route}`)
  await expect(page.locator('.connection-indicator')).toHaveAttribute('data-state', 'connected')
  await expect(shot.ready(page)).toBeVisible()
  if (shot.then) await expect(await shot.then(page)).toBeVisible()
  // Transitions run on drawn frames, which nothing may ask for yet.
  await page.evaluate(() =>
    Promise.all(
      document.getAnimations().flatMap((a) => (a instanceof CSSTransition ? [a.finished] : [])),
    ),
  )
}
