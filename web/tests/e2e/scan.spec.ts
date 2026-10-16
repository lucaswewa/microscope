import { expect, test } from '@playwright/test'

// Checkpoint D: a scan from the Slide Scan tab shows its live stitching
// preview, its images outlined (P53), then ends as a stitched deep-zoom image
// in the gallery, outlined too (P53b); one with autofocus off (P51) takes one
// image a site. They hold the microscope, so they run in turn, once the other
// tests have finished.
test.describe.configure({ mode: 'serial' })
test.use({ viewport: { width: 1280, height: 800 } })

test('a scan from the Slide Scan tab ends stitched in the gallery', async ({ page, request }) => {
  test.setTimeout(240_000)
  // A small, quick snake: 2 × 1 sites, at the stream's size.
  for (const [setting, value] of [
    ['x_count', 2],
    ['y_count', 1],
    ['capture_mode', 'quick'],
  ] as const) {
    const response = await request.put(`/api/v1/snake_workflow/${setting}`, {
      data: JSON.stringify(value),
      headers: { 'Content-Type': 'application/json' },
    })
    expect(response.ok()).toBe(true)
  }
  await page.goto('/#/slide-scan')
  const pane = page.locator('.control-pane')
  await pane.getByRole('combobox').click()
  await page.getByRole('option', { name: 'Snake scan' }).click()
  await expect(pane).toContainText('turning back at the end of each row')

  const name = `e2e-${Date.now()}`
  await pane.getByRole('textbox', { name: 'Sample ID' }).fill(name)
  await pane.getByRole('button', { name: 'Start Smart Scan' }).click()
  await expect(pane.getByRole('button', { name: /^Cancel (setup|scan)$/ })).toBeVisible()
  await expect(page.locator('.slide-scan__preview')).toBeVisible({ timeout: 120_000 })
  await expect(pane.getByText('The scan has finished.')).toBeVisible({ timeout: 120_000 })
  await expect(pane).toContainText(name)
  // Each image outlined over the preview (P53), which the Layers menu turns off and on.
  const outlines = page.locator('.overlay-layers [data-layer="image-outlines"] > g')
  await expect(outlines).toHaveCount(2)
  await expect(outlines.first().locator('title')).toHaveText(/^1: img_/)
  await page.getByRole('button', { name: 'Layers' }).click()
  const choice = page.getByRole('menuitemcheckbox', { name: 'Image outlines' })
  await choice.click()
  await expect(outlines).toHaveCount(0)
  await choice.click()
  await expect(outlines).toHaveCount(2)
  await page.keyboard.press('Escape')
  await pane.getByRole('button', { name: 'Close' }).click()
  await expect(pane.getByRole('button', { name: 'Start Smart Scan' })).toBeVisible()

  // Stitched once it ended, its card shows the mosaic and opens it.
  await page.getByRole('link', { name: 'Gallery' }).click()
  const card = page.locator('.gallery-card', { hasText: name })
  await expect(async () => {
    await page.getByRole('button', { name: 'Refresh' }).click()
    const thumbnail = card.locator('.gallery-card__thumbnail img')
    await expect(thumbnail).toHaveAttribute('src', /stitched\/thumbnail\.jpeg/, { timeout: 2_000 })
  }).toPass({ timeout: 60_000 })
  const mosaic = page.waitForRequest(/stitched\/mosaic\.dzi$/)
  await card.getByRole('button', { name: 'View', exact: true }).click()
  await mosaic
  await expect(page.locator('.deep-zoom')).toHaveAttribute('data-status', 'shown')
  // Its images outlined over it (P53b), following the zoom.
  const dialog = page.getByRole('dialog')
  const strokes = dialog.locator('.overlay-layers__stroke')
  await expect(strokes).toHaveCount(2)
  const width = async () => Number(await strokes.first().getAttribute('width'))
  const before = await width()
  await dialog.getByRole('button', { name: 'Zoom in' }).click()
  await expect.poll(width).toBeGreaterThan(before * 1.4)
})

test('a scan with autofocus off takes one image a site, where the stage is', async ({
  page,
  request,
}) => {
  test.setTimeout(120_000)
  const put = async (path: string, value: unknown) => {
    const response = await request.put(`/api/v1/${path}`, {
      data: JSON.stringify(value),
      headers: { 'Content-Type': 'application/json' },
    })
    expect(response.ok()).toBe(true)
  }
  for (const [setting, value] of [
    ['x_count', 2],
    ['y_count', 1],
    ['capture_mode', 'quick'],
  ] as const) {
    await put(`snake_workflow/${setting}`, value)
  }
  await page.goto('/#/slide-scan')
  const pane = page.locator('.control-pane')
  await pane.getByRole('combobox').click()
  await page.getByRole('option', { name: 'Snake scan' }).click()
  const autofocus = pane.getByRole('switch', { name: 'Autofocus at the start and at each site' })
  await expect(autofocus).toBeChecked()
  await autofocus.uncheck()
  try {
    const name = `e2e-flat-${Date.now()}`
    await pane.getByRole('textbox', { name: 'Sample ID' }).fill(name)
    await pane.getByRole('button', { name: 'Start Smart Scan' }).click()
    await expect(pane.getByText('The scan has finished.')).toBeVisible({ timeout: 60_000 })
    const details = await (await request.get('/api/v1/smart_scan/latest_scan_live_details')).json()
    expect(details).toMatchObject({ name, image_count: 2 })
    expect(details.settings.autofocus_enabled).toBe(false)
    await pane.getByRole('button', { name: 'Close' }).click()
  } finally {
    await put('smart_scan/autofocus_enabled', true)
  }
})
