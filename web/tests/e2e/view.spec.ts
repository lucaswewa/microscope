import { expect, test, type Page } from '@playwright/test'

test.use({ viewport: { width: 1280, height: 800 } })

/** How many frames the live image has drawn. */
const framesDrawn = (page: Page) =>
  page
    .getByRole('img', { name: 'Live image' })
    .getAttribute('data-frames')
    .then((count) => Number(count))

test('the live image shows the camera’s frames', async ({ page }) => {
  await page.goto('/#/view')
  await expect(page.getByRole('img', { name: 'Live image' })).toBeVisible()
  await expect.poll(() => framesDrawn(page)).toBeGreaterThan(3)
  await expect(page.locator('.live-image')).toHaveAttribute('data-status', 'live')
})

test('the stream stops when you leave for a tab without the live image', async ({ page }) => {
  const stopped: string[] = []
  page.on('requestfailed', (request) => {
    if (request.url().endsWith('/camera/mjpeg_stream')) stopped.push(request.url())
  })
  await page.goto('/#/view')
  await expect.poll(() => framesDrawn(page)).toBeGreaterThan(0)
  await page
    .getByRole('navigation', { name: 'Main' })
    .getByRole('link', { name: 'Logging' })
    .click()
  await expect.poll(() => stopped.length).toBe(1)
})

test('the stream can be turned off, and stays off after a reload', async ({ page }) => {
  await page.goto('/#/view')
  await page.getByRole('button', { name: 'Disable stream' }).click()
  await expect(page.locator('.live-image')).toHaveAttribute('data-status', 'disabled')
  await page.reload()
  await expect(page.locator('.live-image')).toHaveAttribute('data-status', 'disabled')
  await page.getByRole('button', { name: 'Turn it on' }).click()
  await expect.poll(() => framesDrawn(page)).toBeGreaterThan(0)
})
