import { expect, test, type Page } from '@playwright/test'

// These tests move the one stage, so they run one at a time.
test.describe.configure({ mode: 'serial' })
test.use({ viewport: { width: 1280, height: 800 } })

type Position = { x: number; y: number; z: number }

const position = async (page: Page): Promise<Position> =>
  (await page.request.get('/api/v1/stage/position')).json()

/** The live image, shrunk to 80 × 60 grey levels. */
const glance = (page: Page) =>
  page.getByRole('img', { name: 'Live image' }).evaluate((canvas: HTMLCanvasElement) => {
    const small = document.createElement('canvas')
    Object.assign(small, { width: 80, height: 60 })
    const context = small.getContext('2d')!
    context.drawImage(canvas, 0, 0, 80, 60)
    const { data } = context.getImageData(0, 0, 80, 60)
    return Array.from({ length: 80 * 60 }, (_, i) => data[i * 4 + 1]!)
  })

/** How different two glances are: the sensor's noise alone gives about 1. */
const difference = (a: number[], b: number[]) =>
  a.reduce((sum, value, i) => sum + Math.abs(value - b[i]!), 0) / a.length

async function openControl(page: Page) {
  await page.goto('/#/control')
  await expect(page.locator('.live-image')).toHaveAttribute('data-status', 'live')
}

test('holding the d-pad moves the stage, and the image follows', async ({ page }) => {
  await openControl(page)
  const before = await position(page)
  const image = await glance(page)
  const right = page.getByRole('button', { name: 'Move right' })
  await right.hover()
  await page.mouse.down()
  await page.waitForTimeout(800)
  await page.mouse.up()
  await expect.poll(async () => (await position(page)).x).toBeGreaterThan(before.x + 400)
  await expect.poll(async () => difference(image, await glance(page))).toBeGreaterThan(5)
})

test('a tap of a key moves one step, and PgDn focuses', async ({ page }) => {
  await openControl(page)
  const before = await position(page)
  await page.locator('.live-image').click()
  // Taps in turn: like any jog, a press replaces the move in progress.
  await page.keyboard.press('ArrowUp')
  await expect.poll(() => position(page)).toEqual({ ...before, y: before.y - 200 })
  await page.keyboard.press('PageDown')
  await expect
    .poll(() => position(page))
    .toEqual({ x: before.x, y: before.y - 200, z: before.z - 50 })
})

test('the wheel over the image focuses, and the image blurs', async ({ page }) => {
  await openControl(page)
  const before = await position(page)
  const image = await glance(page)
  await page.locator('.live-image').hover()
  await page.mouse.wheel(0, -600) // six notches away from you: up 300 steps, 15 µm
  await expect.poll(() => position(page)).toEqual({ ...before, z: before.z + 300 })
  await expect.poll(async () => difference(image, await glance(page))).toBeGreaterThan(5)
})

test('typed coordinates move the stage, and Move Home brings it back', async ({ page }) => {
  await openControl(page)
  const fields = page.locator('.position-section input')
  await fields.nth(0).fill('1500')
  await fields.nth(1).fill('-800')
  await fields.nth(2).fill('20')
  await page.getByRole('button', { name: 'Move', exact: true }).click()
  await expect.poll(() => position(page)).toEqual({ x: 1500, y: -800, z: 20 })
  await expect(page.locator('.position-section__live')).toContainText('x 1500, y -800, z 20')

  await page.getByRole('button', { name: 'Move Home' }).click()
  await page.getByRole('alertdialog').getByRole('button', { name: 'Move Home' }).click()
  await expect.poll(() => position(page), { timeout: 10_000 }).toEqual({ x: 0, y: 0, z: 0 })
})
