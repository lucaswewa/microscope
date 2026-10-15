import { expect, test, type Page } from '@playwright/test'

// Whole workflows against the real server (P48), as someone would go
// through them: a first session, from connecting to finding a capture in
// the gallery, and a histo scan that ends stitched. They move the stage and
// hold the microscope, so they run in turn, once the other tests are done.
test.describe.configure({ mode: 'serial' })
test.use({ viewport: { width: 1280, height: 800 } })

const x = async (page: Page) =>
  ((await (await page.request.get('/api/v1/stage/position')).json()) as { x: number }).x

test('a first session: connect, jog, autofocus, capture, and find it in the gallery', async ({
  page,
}) => {
  test.setTimeout(120_000)
  await page.goto('/')
  await expect(page.locator('.connection-indicator')).toHaveAttribute('data-state', 'connected')
  await page.getByRole('link', { name: 'Control' }).click()
  await expect(page.locator('.live-image')).toHaveAttribute('data-status', 'live')

  const before = await x(page)
  await page.getByRole('button', { name: 'Move right' }).click()
  await expect.poll(() => x(page)).toBeGreaterThan(before)

  await page.getByRole('button', { name: 'Autofocus', exact: true }).click()
  await expect(page.getByRole('button', { name: 'Cancel autofocus' })).toBeVisible()
  await expect(page.getByRole('button', { name: 'Autofocus', exact: true })).toBeVisible({
    timeout: 60_000,
  })

  await page.getByRole('button', { name: 'Capture' }).click()
  await expect(page.getByText('Saved to the gallery.').first()).toBeVisible({ timeout: 20_000 })
  await page.getByRole('link', { name: 'Gallery' }).click()
  const card = page.locator('.gallery-card').first()
  await expect(card.locator('.gallery-card__title')).toHaveText(/^capture-/)
  await card.getByRole('button', { name: 'View', exact: true }).click()
  await expect(page.locator('.deep-zoom')).toHaveAttribute('data-status', 'shown')
})

test('a histo scan from the Slide Scan tab ends stitched in the gallery', async ({
  page,
  request,
}) => {
  test.setTimeout(300_000)
  // A short one: no background to skip, a small range, and quick captures.
  for (const [setting, value] of [
    ['skip_background', false],
    ['max_range', 1000],
    ['capture_mode', 'quick'],
    ['stack_min_images_to_test', 3],
  ] as const) {
    const response = await request.put(`/api/v1/histo_scan_workflow/${setting}`, {
      data: JSON.stringify(value),
      headers: { 'Content-Type': 'application/json' },
    })
    expect(response.ok(), setting).toBe(true)
  }
  await page.goto('/#/slide-scan')
  const pane = page.locator('.control-pane')
  await pane.getByRole('combobox').click()
  await page.getByRole('option', { name: 'Histo scan' }).click()
  await expect(pane).toContainText('spirals out from where the stage is')
  const name = `histo-${Date.now()}`
  await pane.getByRole('textbox', { name: 'Sample ID' }).fill(name)
  await pane.getByRole('button', { name: 'Start Smart Scan' }).click()
  await expect(page.locator('.slide-scan__preview')).toBeVisible({ timeout: 120_000 })
  await expect(pane.getByText('The scan has finished.')).toBeVisible({ timeout: 240_000 })

  await page.getByRole('link', { name: 'Gallery' }).click()
  const card = page.locator('.gallery-card', { hasText: name })
  await expect(async () => {
    await page.getByRole('button', { name: 'Refresh' }).click()
    const thumbnail = card.locator('.gallery-card__thumbnail img')
    await expect(thumbnail).toHaveAttribute('src', /stitched\/thumbnail\.jpeg/, { timeout: 2_000 })
  }).toPass({ timeout: 60_000 })
})
