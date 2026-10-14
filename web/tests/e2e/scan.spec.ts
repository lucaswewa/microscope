import { expect, test } from '@playwright/test'

// Checkpoint D: a scan from the Slide Scan tab shows its live stitching
// preview, then ends as a stitched deep-zoom image in the gallery. It holds
// the microscope for a while, so it runs once the other tests have finished.
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
})
